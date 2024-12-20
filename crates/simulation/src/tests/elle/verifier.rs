use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    io::Write,
};

use anyhow::Context;

use super::{
    event::ElleModelEvent,
    ClientId,
    TxId,
    WriteId,
    SERVER_CLIENT_ID,
};

pub enum Node {
    ReadTx {
        tx_id: TxId,
        client_id: ClientId,
        write_ids: Option<Vec<WriteId>>,
    },
    WriteTx {
        tx_id: TxId,
        client_id: ClientId,
        write_ids: Vec<WriteId>,
    },
}

impl Node {
    pub fn tx_id(&self) -> TxId {
        match self {
            Node::ReadTx { tx_id, .. } => *tx_id,
            Node::WriteTx { tx_id, .. } => *tx_id,
        }
    }

    pub fn client_id(&self) -> ClientId {
        match self {
            Node::ReadTx { client_id, .. } => *client_id,
            Node::WriteTx { client_id, .. } => *client_id,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Node::ReadTx { .. } => "R",
            Node::WriteTx { .. } => "W",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeType {
    DirectWriteDepends,
    DirectReadDepends,
    AntiReadDepends,

    ClientWriteOrder,
    ClientReadOrder,
}

impl EdgeType {
    pub fn label(&self) -> &'static str {
        match self {
            EdgeType::DirectWriteDepends => "DW",
            EdgeType::DirectReadDepends => "DR",
            EdgeType::AntiReadDepends => "AR",
            EdgeType::ClientWriteOrder => "CW",
            EdgeType::ClientReadOrder => "CR",
        }
    }
}

pub struct ElleVerifier {
    nodes: Vec<Node>,
    node_by_tx_id: BTreeMap<TxId, usize>,

    edges: BTreeSet<(usize, usize, EdgeType)>,
}

impl ElleVerifier {
    pub fn new(event_log: &[ElleModelEvent]) -> anyhow::Result<Self> {
        let (nodes, node_by_tx_id) = Self::build_nodes(event_log)?;
        let mut this = Self {
            nodes,
            node_by_tx_id,
            edges: BTreeSet::new(),
        };
        this.build_edges(event_log)?;
        Ok(this)
    }

    fn build_nodes(
        event_log: &[ElleModelEvent],
    ) -> anyhow::Result<(Vec<Node>, BTreeMap<TxId, usize>)> {
        let mut in_progress_txes = BTreeMap::new();
        // Start with a single node for the server's initial state.
        let mut nodes = vec![Node::WriteTx {
            tx_id: 0,
            client_id: SERVER_CLIENT_ID,
            write_ids: vec![],
        }];
        let mut node_by_tx_id = BTreeMap::new();
        for event in event_log {
            match event {
                ElleModelEvent::StartRead { tx_id, .. }
                | ElleModelEvent::StartWrite { tx_id, .. } => {
                    in_progress_txes.insert(*tx_id, event);
                },
                ElleModelEvent::FinishRead {
                    client_id,
                    tx_id,
                    write_ids,
                    ..
                } => {
                    let Some(tx) = in_progress_txes.remove(tx_id) else {
                        anyhow::bail!("tx not in progress");
                    };
                    let ElleModelEvent::StartRead { .. } = tx else {
                        anyhow::bail!("tx is not a read tx");
                    };
                    let node = Node::ReadTx {
                        tx_id: *tx_id,
                        client_id: *client_id,
                        write_ids: write_ids.clone(),
                    };
                    let node_ix = nodes.len();
                    nodes.push(node);
                    node_by_tx_id.insert(*tx_id, node_ix);
                },
                ElleModelEvent::FinishWrite {
                    client_id,
                    tx_id,
                    write_ids,
                    ..
                } => {
                    let Some(tx) = in_progress_txes.remove(tx_id) else {
                        anyhow::bail!("tx not in progress");
                    };
                    let ElleModelEvent::StartWrite { .. } = tx else {
                        anyhow::bail!("tx is not a write tx");
                    };
                    let node = Node::WriteTx {
                        tx_id: *tx_id,
                        client_id: *client_id,
                        write_ids: write_ids.clone(),
                    };
                    let node_ix = nodes.len();
                    nodes.push(node);
                    node_by_tx_id.insert(*tx_id, node_ix);
                },
            }
        }
        Ok((nodes, node_by_tx_id))
    }

    fn build_edges(&mut self, event_log: &[ElleModelEvent]) -> anyhow::Result<()> {
        // Index the txid for the write transaction that installed a given length.
        let mut register_len_index = BTreeMap::new();

        // Index the txid for the write transaction that installed a given write_id.
        let mut txid_by_write_id = BTreeMap::new();

        let root_write_tx_id = self.nodes[0].tx_id();
        let Node::WriteTx { write_ids, .. } = &self.nodes[root_write_tx_id] else {
            anyhow::bail!("root node is not a write tx");
        };
        anyhow::ensure!(write_ids.is_empty(), "root write_ids is not empty");

        for node in &self.nodes {
            let Node::WriteTx {
                tx_id, write_ids, ..
            } = node
            else {
                continue;
            };
            let Some(write_id) = write_ids.last() else {
                continue;
            };
            anyhow::ensure!(register_len_index.insert(write_ids.len(), *tx_id).is_none());
            anyhow::ensure!(txid_by_write_id.insert(*write_id, *tx_id).is_none());
        }

        // Direct write dependency: tx_i directly write depends on tx_j when tx_i
        // installs x_i, and tx_j installs the next version.
        for (i, node) in self.nodes.iter().enumerate() {
            let Node::WriteTx { write_ids, .. } = node else {
                continue;
            };
            if let Some(next_tx_id) = register_len_index.get(&(write_ids.len() + 1)) {
                let j = self.node_by_tx_id[next_tx_id];
                self.edges.insert((i, j, EdgeType::DirectWriteDepends));
            }
        }

        // Direct read dependency: tx_i directly read depends on tx_j when tx_i
        // installs x_i, and tx_j reads x_i.
        for (i, node) in self.nodes.iter().enumerate() {
            let Node::ReadTx {
                write_ids: Some(write_ids),
                ..
            } = node
            else {
                continue;
            };
            let j = match write_ids.last() {
                Some(write_id) => {
                    let write_tx_id = txid_by_write_id
                        .get(write_id)
                        .context("write_txid not found")?;
                    self.node_by_tx_id[write_tx_id]
                },
                None => root_write_tx_id,
            };
            self.edges.insert((j, i, EdgeType::DirectReadDepends));
        }

        // Read anti-dependency: tx_i precedes tx_j when tx_i reads x_i, and tx_j
        // installs the next version.
        for (i, node) in self.nodes.iter().enumerate() {
            let Node::ReadTx {
                write_ids: Some(write_ids),
                ..
            } = node
            else {
                continue;
            };
            let Some(write_tx_id) = register_len_index.get(&(write_ids.len() + 1)) else {
                continue;
            };
            let j = self.node_by_tx_id[write_tx_id];
            self.edges.insert((i, j, EdgeType::AntiReadDepends));
        }

        // Each client's writes must be fully serialized by their initialization point.
        let mut last_write_start = BTreeMap::new();

        // Each client's reads must come after their last completed write.
        let mut last_write_end = BTreeMap::new();
        for event in event_log {
            match event {
                ElleModelEvent::StartWrite {
                    client_id, tx_id, ..
                } => {
                    if let Some(last_tx_id) = last_write_start.insert(*client_id, *tx_id) {
                        let i = self.node_by_tx_id[&last_tx_id];
                        let j = self.node_by_tx_id[tx_id];
                        self.edges.insert((i, j, EdgeType::ClientWriteOrder));
                    }
                },
                ElleModelEvent::FinishWrite {
                    client_id, tx_id, ..
                } => {
                    last_write_end.insert(*client_id, *tx_id);
                },
                ElleModelEvent::StartRead {
                    tx_id, client_id, ..
                } => {
                    if let Some(last_tx_id) = last_write_end.get(client_id) {
                        let i = self.node_by_tx_id[last_tx_id];
                        let j = self.node_by_tx_id[tx_id];
                        self.edges.insert((i, j, EdgeType::ClientReadOrder));
                    }
                },
                _ => {},
            }
        }

        Ok(())
    }

    pub fn verify_acyclic(&self) -> anyhow::Result<()> {
        let mut roots: BTreeSet<_> = (0..self.nodes.len()).collect();
        for (_, end, _) in &self.edges {
            roots.remove(end);
        }
        anyhow::ensure!(!roots.is_empty(), "no roots found");

        let mut stack: Vec<_> = roots.into_iter().map(|n| (n, true)).collect();

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum NodeState {
            Unvisited,
            InProgress,
            Done,
        }
        let mut node_state = vec![NodeState::Unvisited; self.nodes.len()];

        while let Some((node, first)) = stack.pop() {
            if node_state[node] == NodeState::Done {
                continue;
            }
            if first {
                anyhow::ensure!(
                    node_state[node] == NodeState::Unvisited,
                    "cycle detected: {stack:#?}"
                );
                node_state[node] = NodeState::InProgress;
                stack.push((node, false));
                let iter = self
                    .edges
                    .range((node, 0usize, EdgeType::DirectWriteDepends)..)
                    .take_while(|(other_node, ..)| node == *other_node);
                for (_, child, _) in iter {
                    stack.push((*child, true));
                }
            } else {
                node_state[node] = NodeState::Done;
            }
        }

        Ok(())
    }

    pub fn render_graphviz(&self, out: &mut impl Write) -> anyhow::Result<()> {
        writeln!(out, "digraph G {{")?;

        for node in &self.nodes {
            writeln!(
                out,
                "  tx{} [label=\"{}({})\"];",
                node.tx_id(),
                node.label(),
                node.client_id()
            )?;
        }
        for (start, end, edge_type) in &self.edges {
            let start_node = &self.nodes[*start];
            let end_node = &self.nodes[*end];
            writeln!(
                out,
                "  tx{} -> tx{} [label=\"{}\"];",
                start_node.tx_id(),
                end_node.tx_id(),
                edge_type.label()
            )?;
        }

        writeln!(out, "}}")?;
        Ok(())
    }
}
