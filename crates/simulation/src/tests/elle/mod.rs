mod config;
mod event;
mod verifier;

use std::{
    cmp,
    collections::BTreeMap,
    env,
    fs::File,
    time::Duration,
};

use common::{
    assert_obj,
    knobs::RUNTIME_STACK_SIZE,
    runtime::Runtime,
    value,
};
use config::ElleConfig;
use event::ElleModelEvent;
use rand::{
    distributions::WeightedIndex,
    prelude::Distribution,
    Rng,
    RngCore,
};
use rand_distr::Geometric;
use runtime::testing::TestDriver;
use tokio::task::JoinSet;
use verifier::ElleVerifier;

use crate::test_helpers::{
    js_client::QueryToken,
    simulation::{
        SimulationTest,
        SimulationTestConfig,
    },
};

type ClientId = usize;
type RegisterId = String;
type WriteId = u32;
type TxId = usize;

const SERVER_CLIENT_ID: ClientId = 3490524077;

struct ElleSimulationTest {
    test: SimulationTest,
    config: ElleConfig,
    rng: Box<dyn RngCore>,

    join_set: JoinSet<anyhow::Result<ElleModelEvent>>,
    next_write_id: WriteId,
    next_tx_id: TxId,
    event_log: Vec<ElleModelEvent>,

    disconnected_clients: BTreeMap<ClientId, usize>,
}

impl ElleSimulationTest {
    pub fn new(test: SimulationTest, config: ElleConfig) -> Self {
        let rng = test.rt.rng();
        Self {
            test,
            config,
            rng,
            next_write_id: 0,
            next_tx_id: 1,
            event_log: vec![],
            join_set: JoinSet::new(),
            disconnected_clients: BTreeMap::new(),
        }
    }

    async fn create_register(&mut self) -> anyhow::Result<RegisterId> {
        let register_id: String = self
            .test
            .server
            .mutation("elle:initializeRegister".parse()?, assert_obj!())
            .await??
            .value
            .try_into()?;
        Ok(register_id)
    }

    async fn add_query(
        &mut self,
        register_id: RegisterId,
        client_id: ClientId,
    ) -> anyhow::Result<QueryToken> {
        let token = self.test.js_clients[client_id]
            .add_query(
                "elle:getRegister".parse()?,
                assert_obj!("id" => register_id),
            )
            .await?;
        Ok(token)
    }

    fn start_read(&mut self, tx_id: TxId, client_id: ClientId, token: QueryToken) {
        let js_client = self.test.js_clients[client_id].clone();
        let future = async move {
            let write_ids: Option<Vec<f64>> = js_client
                .query_result(token)
                .await?
                .map(value::serde::from_value)
                .transpose()?;
            Ok(ElleModelEvent::FinishRead {
                tx_id,
                client_id,
                write_ids: write_ids.map(|w| w.iter().map(|w| *w as WriteId).collect()),
            })
        };
        let start = ElleModelEvent::StartRead { tx_id, client_id };
        self.event_log.push(start);
        self.join_set.spawn(future);
    }

    fn start_client_write(&mut self, tx_id: TxId, client_id: ClientId, register_id: RegisterId) {
        let write_id = self.next_write_id;
        self.next_write_id += 1;

        let js_client = self.test.js_clients[client_id].clone();
        let future = async move {
            let result = js_client
                .run_mutation(
                    "elle:appendRegister".parse()?,
                    assert_obj!("id" => register_id, "value" => write_id as f64),
                )
                .await??;
            let write_ids: Vec<f64> = value::serde::from_value(result)?;
            let write_ids: Vec<WriteId> = write_ids.iter().map(|w| *w as WriteId).collect();
            anyhow::ensure!(write_ids.last() == Some(&write_id));
            Ok(ElleModelEvent::FinishWrite {
                tx_id,
                client_id,
                write_ids,
            })
        };
        let start = ElleModelEvent::StartWrite {
            tx_id,
            client_id,
            write_id,
        };
        self.event_log.push(start);
        self.join_set.spawn(future);
    }

    fn start_server_write(&mut self, tx_id: TxId, register_id: RegisterId) {
        let write_id = self.next_write_id;
        self.next_write_id += 1;

        let client_id = SERVER_CLIENT_ID;
        let server = self.test.server.clone();
        let args = assert_obj!("id" => register_id, "value" => write_id as f64);
        let future = async move {
            let result = server
                .mutation("elle:appendRegister".parse()?, args)
                .await??;
            let write_ids: Vec<f64> = value::serde::from_value(result.value)?;
            let write_ids: Vec<WriteId> = write_ids.iter().map(|w| *w as WriteId).collect();
            Ok(ElleModelEvent::FinishWrite {
                tx_id,
                client_id,
                write_ids,
            })
        };
        let start = ElleModelEvent::StartWrite {
            tx_id,
            client_id,
            write_id,
        };
        self.event_log.push(start);
        self.join_set.spawn(future);
    }

    async fn run(mut self) -> anyhow::Result<Vec<ElleModelEvent>> {
        let register_id = self.create_register().await?;

        let mut tokens = vec![];
        for i in 0..self.config.num_clients {
            tokens.push(self.add_query(register_id.clone(), i).await?);
        }

        let actions = [
            Action::ClientRead,
            Action::ClientWrite,
            Action::ServerWrite,
            Action::DisconnectClient,
        ];
        let dist = WeightedIndex::new([
            self.config.client_read_weight,
            self.config.client_write_weight,
            self.config.server_write_weight,
            self.config.disconnect_client_weight,
        ])?;
        let duration_dist =
            Geometric::new(1.0 / (self.config.expected_disconnect_duration as f64))?;

        loop {
            // Kick off new work if we have room.
            while self.next_tx_id < self.config.num_tx
                && self.join_set.len() < self.config.max_concurrent_tx
            {
                let tx_id = self.next_tx_id;
                self.next_tx_id += 1;

                let mut to_reconnect = vec![];
                for (client_id, duration) in self.disconnected_clients.iter_mut() {
                    *duration -= 1;
                    if *duration == 0 {
                        to_reconnect.push(*client_id);
                    }
                }
                for client_id in to_reconnect {
                    self.test.js_clients[client_id].reconnect_network().await?;
                    self.disconnected_clients.remove(&client_id);
                }

                match actions[dist.sample(&mut self.rng)] {
                    Action::ClientRead => {
                        let client_id = self.rng.gen_range(0..self.config.num_clients);
                        self.start_read(tx_id, client_id, tokens[client_id].clone());
                    },
                    Action::ClientWrite => {
                        let client_id = self.rng.gen_range(0..self.config.num_clients);
                        self.start_client_write(tx_id, client_id, register_id.clone());
                    },
                    Action::ServerWrite => {
                        self.start_server_write(tx_id, register_id.clone());
                    },
                    Action::DisconnectClient => {
                        let client_id = self.rng.gen_range(0..self.config.num_clients);
                        let remaining_tx = self.config.num_tx - self.next_tx_id;
                        let duration =
                            cmp::min(duration_dist.sample(&mut self.rng) as usize, remaining_tx);
                        if duration > 0 {
                            self.test.js_clients[client_id].disconnect_network().await?;
                            self.disconnected_clients.insert(client_id, duration);
                        }
                    },
                }
            }

            // Block on the next event, finishing the test if the set is empty.
            let Some(result) = self.join_set.join_next().await else {
                break;
            };
            let event = result??;
            self.event_log.push(event);
        }

        Ok(self.event_log)
    }
}

enum Action {
    ClientRead,
    ClientWrite,
    ServerWrite,
    DisconnectClient,
}

#[test]
fn test_elle_model() -> anyhow::Result<()> {
    let thread_handle = std::thread::Builder::new()
        .stack_size(*RUNTIME_STACK_SIZE)
        .spawn(|| {
            let config = ElleConfig::default();
            let td = TestDriver::new_with_seed(config.seed);
            let future = SimulationTest::run(
                td.rt(),
                SimulationTestConfig {
                    num_client_threads: config.num_clients,
                    expected_delay_duration: Some(Duration::from_secs(1)),
                },
                async move |t: SimulationTest| {
                    let sim = ElleSimulationTest::new(t, config);
                    let event_log = sim.run().await?;

                    let verifier = ElleVerifier::new(&event_log)?;

                    if let Ok(path) = env::var("ELLE_DOT_PATH") {
                        let mut f = File::create(path)?;
                        verifier.render_graphviz(&mut f)?;
                    }

                    verifier.verify_acyclic()?;

                    Ok(())
                },
            );
            td.run_until(future)?;
            anyhow::Ok(())
        })?;
    thread_handle.join().expect("thread panicked")?;
    Ok(())
}
