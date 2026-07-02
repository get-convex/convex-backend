import { useCallback, useEffect, useMemo, useRef, useState } from "react";
// Imported here (not via globals.css) because the relative @import there was
// silently dropped in production builds, hiding all nodes and the grid.
import "@xyflow/react/dist/style.css";
import {
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useNodesState,
} from "@xyflow/react";
import { SchemaGraph as SchemaGraphData } from "@common/features/schema/lib/buildSchemaGraph";
import {
  NodePositions,
  NodeSize,
  NODE_WIDTH,
  nodeSize,
  mergeSavedLayout,
} from "@common/features/schema/lib/layout";
import {
  SchemaSearch,
  buildSearchEntries,
} from "@common/features/schema/components/SchemaSearch";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useMediaQuery } from "@common/lib/useMediaQuery";
import { computeElkLayout } from "@common/features/schema/lib/elkLayout";
import {
  nodeTypes,
  nodeAriaLabel,
} from "@common/features/schema/components/TableNode";
import { edgeTypes } from "@common/features/schema/components/SchemaEdge";
import { AdaptiveBackground } from "@common/features/schema/components/SchemaBackground";
import { MinimapOverlay } from "@common/features/schema/components/SchemaMinimap";
import { SchemaControls } from "@common/features/schema/components/SchemaControls";
import {
  HoverTarget,
  TableFlowNode,
  ElkFlowEdge,
} from "@common/features/schema/components/schemaFlowTypes";

const VISIBLE_CULL_THRESHOLD = 80;

const INITIAL_FIT_VIEW = { padding: 0.15, minZoom: 0.4 };

// Stable empty set so unhighlighted nodes keep a constant data reference.
const NO_HIGHLIGHTS: ReadonlySet<string> = new Set();

const EDGE_STROKE_ACTIVE = "var(--color-util-accent)";
const EDGE_STROKE_INACTIVE =
  "color-mix(in srgb, var(--color-content-tertiary) 60%, var(--color-background-primary))";

const EDGE_DATA_ACTIVE: ElkFlowEdge["data"] = { active: true };
const EDGE_DATA_INACTIVE: ElkFlowEdge["data"] = { active: false };
const EDGE_STYLE = {
  activeSolid: { stroke: EDGE_STROKE_ACTIVE, strokeWidth: 2 },
  activeDashed: {
    stroke: EDGE_STROKE_ACTIVE,
    strokeWidth: 2,
    strokeDasharray: "5 4",
  },
  inactiveSolid: { stroke: EDGE_STROKE_INACTIVE, strokeWidth: 1.5 },
  inactiveDashed: {
    stroke: EDGE_STROKE_INACTIVE,
    strokeWidth: 1.5,
    strokeDasharray: "5 4",
  },
};

function SchemaFlowInner({
  graph,
  storageKey,
  selectedTable,
  focusRequest,
  onSelectNode,
  onFocusTable,
  onClearSelection,
}: {
  graph: SchemaGraphData;
  storageKey: string;
  selectedTable: string | null;
  // Bump-on-each-request signal to pan a table into view (e.g. a side-panel
  // reference link). The nonce lets repeated requests for the same table re-fire.
  focusRequest: { table: string; nonce: number } | null;
  onSelectNode: (table: string, opts?: { fromKeyboard?: boolean }) => void;
  onFocusTable: (table: string) => void;
  onClearSelection: () => void;
}) {
  const { fitView, getNodes, setCenter, getViewport } = useReactFlow();

  const isTouch = useMediaQuery("(pointer: coarse)");

  const sizes = useMemo<Record<string, NodeSize>>(() => {
    const result: Record<string, NodeSize> = {};
    graph.nodes.forEach((node) => {
      result[node.table] = nodeSize(node.fields.length, node.indexes.length);
    });
    return result;
  }, [graph]);

  // Tables each table references (from edge source -> target), for node labels.
  const referencesByTable = useMemo<Map<string, string[]>>(() => {
    const map = new Map<string, string[]>();
    graph.edges.forEach((edge) => {
      const label =
        edge.source === edge.target ? `${edge.target} (itself)` : edge.target;
      const existing = map.get(edge.source);
      if (existing) {
        if (!existing.includes(label)) {
          existing.push(label);
        }
      } else {
        map.set(edge.source, [label]);
      }
    });
    return map;
  }, [graph.edges]);

  // Persisted under its own key, independent of the force-directed view's layout.
  const [savedPositions, setSavedPositions, clearSavedPositions] =
    useGlobalLocalStorage<NodePositions>(`${storageKey}/elk`, {});
  const savedPositionsRef = useRef(savedPositions);
  savedPositionsRef.current = savedPositions;

  const [nodes, setNodes, onNodesChange] = useNodesState<TableFlowNode>([]);
  const [hover, setHover] = useState<HoverTarget | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);

  const searchEntries = useMemo(() => buildSearchEntries(graph), [graph]);

  const pendingFocusRef = useRef<string | null>(null);
  const handledFocusNonceRef = useRef<number | null>(null);

  const buildNodes = useCallback(
    (positions: NodePositions): TableFlowNode[] =>
      graph.nodes.map((node) => {
        const size = sizes[node.table];
        return {
          id: node.table,
          type: "table",
          position: positions[node.table] ?? { x: 0, y: 0 },
          // Seed the estimated size so culling has bounds before React Flow
          // measures the node.
          initialWidth: size?.width,
          initialHeight: size?.height,
          // Spoken name for screen readers (read verbatim as the aria-label).
          ariaLabel: nodeAriaLabel(
            node,
            referencesByTable.get(node.table) ?? [],
          ),
          className:
            "rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-util-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-primary",
          data: {
            node,
            // Selection and highlights are overlaid at render by
            // `decoratedNodes`, so seed them empty here. Keeping selection out
            // of `buildNodes` means its identity only changes with the graph
            // (not on every table click), so the ELK effect below can depend on
            // it without re-running the layout on each selection.
            isSelected: false,
            highlightedFields: NO_HIGHLIGHTS,
            highlightedIndexes: NO_HIGHLIGHTS,
            onHover: setHover,
          },
        };
      }),
    [graph.nodes, sizes, referencesByTable],
  );

  // Re-run ELK whenever the graph changes, merging any saved manual layout.
  useEffect(() => {
    let cancelled = false;
    void computeElkLayout(graph, sizes).then(({ positions }) => {
      if (cancelled) return;
      const merged = mergeSavedLayout(
        savedPositionsRef.current,
        positions,
        sizes,
      );
      setNodes(buildNodes(merged));
      window.requestAnimationFrame(() => {
        // A focus-into-view request owns the viewport; don't fit over it.
        if (pendingFocusRef.current) return;
        void fitView(INITIAL_FIT_VIEW);
      });
    });
    return () => {
      cancelled = true;
    };
  }, [graph, sizes, buildNodes, fitView, setNodes]);

  const centerOnTable = useCallback(
    (table: string): boolean => {
      const node = getNodes().find((n) => n.id === table);
      if (!node) {
        return false;
      }
      const size = sizes[table];
      const centerX = node.position.x + (size?.width ?? NODE_WIDTH) / 2;
      const centerY = node.position.y + (size?.height ?? 0) / 2;
      void setCenter(centerX, centerY, {
        zoom: getViewport().zoom,
        duration: 400,
      });
      return true;
    },
    [getNodes, sizes, setCenter, getViewport],
  );

  useEffect(() => {
    if (!focusRequest || handledFocusNonceRef.current === focusRequest.nonce) {
      return;
    }
    // A table absent from the graph (e.g. a reference to a since-deleted table)
    // will never be placed. Mark the request handled and leave the pending ref
    // clear so the layout effect's fitView isn't blocked waiting for it.
    if (!graph.nodes.some((n) => n.table === focusRequest.table)) {
      handledFocusNonceRef.current = focusRequest.nonce;
      pendingFocusRef.current = null;
      return;
    }
    pendingFocusRef.current = focusRequest.table;
    if (!centerOnTable(focusRequest.table)) {
      // Node not placed yet; retry on the next `nodes` update.
      return;
    }
    handledFocusNonceRef.current = focusRequest.nonce;
    // Release the layout effect's fit-to-view a frame later: ELK's own
    // requestAnimationFrame(fitView) runs first and still sees the pending focus.
    window.requestAnimationFrame(() => {
      pendingFocusRef.current = null;
    });
  }, [focusRequest, nodes, centerOnTable, graph.nodes]);

  const highlights = useMemo(() => {
    const fields = new Map<string, Set<string>>();
    const indexes = new Map<string, Set<string>>();
    const addTo = (
      map: Map<string, Set<string>>,
      table: string,
      name: string,
    ) => {
      const existing = map.get(table);
      if (existing) {
        existing.add(name);
      } else {
        map.set(table, new Set([name]));
      }
    };
    if (hover?.kind === "field") {
      addTo(fields, hover.table, hover.field);
    } else if (hover?.kind === "index") {
      addTo(indexes, hover.table, hover.index);
    } else if (hover?.kind === "header") {
      graph.edges.forEach((edge) => {
        if (edge.target === hover.table) {
          addTo(fields, edge.source, edge.field);
        }
      });
    }
    if (selectedTable) {
      graph.edges.forEach((edge) => {
        if (edge.source === selectedTable) {
          addTo(fields, edge.source, edge.field);
        }
      });
    }
    return { fields, indexes };
  }, [graph.edges, hover, selectedTable]);

  // Selection and highlight state are pure derivations of `selectedTable` +
  // `highlights`, so overlay them onto the layout nodes at render rather than
  // writing them back into node state via an effect (which forced an extra
  // render on every hover/select). The identity short-circuit keeps unchanged
  // nodes referentially stable so React Flow skips re-rendering them.
  const decoratedNodes = useMemo<TableFlowNode[]>(
    () =>
      nodes.map((n) => {
        const isSelected = n.id === selectedTable;
        const highlightedFields = highlights.fields.get(n.id) ?? NO_HIGHLIGHTS;
        const highlightedIndexes =
          highlights.indexes.get(n.id) ?? NO_HIGHLIGHTS;
        if (
          n.data.isSelected === isSelected &&
          n.data.highlightedFields === highlightedFields &&
          n.data.highlightedIndexes === highlightedIndexes
        ) {
          return n;
        }
        return {
          ...n,
          data: {
            ...n.data,
            isSelected,
            highlightedFields,
            highlightedIndexes,
          },
        };
      }),
    [nodes, selectedTable, highlights],
  );

  const edges = useMemo<ElkFlowEdge[]>(
    () =>
      graph.edges.map((edge) => {
        const hoverActive =
          hover?.kind === "field"
            ? edge.source === hover.table && edge.field === hover.field
            : hover?.kind === "header"
              ? edge.target === hover.table
              : false;
        const active =
          hoverActive ||
          (selectedTable !== null &&
            (edge.source === selectedTable || edge.target === selectedTable));
        let style: ElkFlowEdge["style"];
        if (active) {
          style = edge.optional
            ? EDGE_STYLE.activeDashed
            : EDGE_STYLE.activeSolid;
        } else {
          style = edge.optional
            ? EDGE_STYLE.inactiveDashed
            : EDGE_STYLE.inactiveSolid;
        }
        return {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          type: "elk",
          data: active ? EDGE_DATA_ACTIVE : EDGE_DATA_INACTIVE,
          style,
        };
      }),
    [graph.edges, hover, selectedTable],
  );

  const persistPositions = useCallback(() => {
    const positions: NodePositions = {};
    getNodes().forEach((n) => {
      positions[n.id] = n.position;
    });
    setSavedPositions(positions);
  }, [getNodes, setSavedPositions]);

  const resetLayout = useCallback(() => {
    clearSavedPositions();
    void computeElkLayout(graph, sizes).then(({ positions }) => {
      setNodes(buildNodes(positions));
      window.requestAnimationFrame(() => fitView(INITIAL_FIT_VIEW));
    });
  }, [clearSavedPositions, graph, sizes, buildNodes, setNodes, fitView]);

  // Cull offscreen elements on large graphs only.
  const cullOffscreen =
    graph.nodes.length + graph.edges.length > VISIBLE_CULL_THRESHOLD;

  return (
    <ReactFlow
      nodes={decoratedNodes}
      edges={edges}
      onlyRenderVisibleElements={cullOffscreen}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodesChange={onNodesChange}
      onNodeClick={(_, node) => onSelectNode(node.id)}
      onNodeDragStop={persistPositions}
      // Only fires while focus is within the flow, so it doesn't hijack keys
      // elsewhere on the page.
      onKeyDown={(e) => {
        if (e.key === "Escape" && selectedTable) {
          onClearSelection();
          return;
        }
        // Open the focused table with Enter/Space, mirroring a click: React Flow
        // toggles its own selection on these keys but never fires onNodeClick.
        // Act only when the node wrapper itself is focused.
        if (e.key === "Enter" || e.key === " ") {
          const active = document.activeElement;
          if (
            active instanceof HTMLElement &&
            active.classList.contains("react-flow__node") &&
            active.dataset.id
          ) {
            onSelectNode(active.dataset.id, { fromKeyboard: true });
          }
        }
      }}
      minZoom={0.15}
      maxZoom={2.5}
      nodesDraggable={!isTouch}
      nodesConnectable={false}
      // Tab between tables; arrow keys move the focused one (React Flow built-in).
      // Edges stay out of the tab order - too many, and references are already
      // spoken in the node label.
      nodesFocusable
      edgesFocusable={false}
      // Mark as an application region so a screen reader passes arrow keys
      // through to move the focused table rather than capturing them.
      aria-label="Database schema diagram. Press Tab to move between tables, then use the arrow keys to reposition the focused table and Enter to open its details."
      role="application"
      // Read-only viewer: never delete nodes via the keyboard.
      deleteKeyCode={null}
      proOptions={{ hideAttribution: true }}
      fitView
      fitViewOptions={INITIAL_FIT_VIEW}
      className="bg-background-primary"
    >
      <AdaptiveBackground />
      <SchemaSearch
        entries={searchEntries}
        onPick={onFocusTable}
        onOpenChange={setSearchOpen}
      />
      {!searchOpen && <MinimapOverlay nodes={decoratedNodes} edges={edges} />}
      <SchemaControls onResetLayout={resetLayout} />
    </ReactFlow>
  );
}

export function SchemaFlow(props: {
  graph: SchemaGraphData;
  storageKey: string;
  selectedTable: string | null;
  focusRequest: { table: string; nonce: number } | null;
  onSelectNode: (table: string, opts?: { fromKeyboard?: boolean }) => void;
  onFocusTable: (table: string) => void;
  onClearSelection: () => void;
}) {
  return (
    <ReactFlowProvider>
      <SchemaFlowInner {...props} />
    </ReactFlowProvider>
  );
}
