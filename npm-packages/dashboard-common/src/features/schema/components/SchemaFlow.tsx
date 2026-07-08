import { useCallback, useEffect, useMemo, useRef, useState } from "react";
// Imported here (not via globals.css) because the relative @import there was
// silently dropped in production builds, hiding all nodes and the grid.
import "@xyflow/react/dist/style.css";
import {
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useNodesState,
  useStoreApi,
  getNodesBounds,
  getViewportForBounds,
} from "@xyflow/react";
import { cn } from "@ui/cn";
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
import { AdaptiveBackground } from "@common/features/schema/components/SchemaBackground";
import {
  SchemaEdge,
  CanvasEdge,
} from "@common/features/schema/components/SchemaEdge";
import { MinimapOverlay } from "@common/features/schema/components/SchemaMinimap";
import { SchemaControls } from "@common/features/schema/components/SchemaControls";
import {
  HoverTarget,
  TableFlowNode,
} from "@common/features/schema/components/schemaFlowTypes";

const VISIBLE_CULL_THRESHOLD = 80;

const INITIAL_FIT_VIEW = { padding: 0.15, minZoom: 0.4 };
const MIN_ZOOM = 0.15;
const MAX_ZOOM = 2.5;

// Stable empty set so unhighlighted nodes keep a constant data reference.
const NO_HIGHLIGHTS: ReadonlySet<string> = new Set();

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
  const { fitView, getNodes, setCenter, getViewport, setViewport } =
    useReactFlow();
  const storeApi = useStoreApi();

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
          className: cn(
            "rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-util-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-primary",
            // Promote each node to its own compositing layer. WebKit (unlike
            // Chrome) doesn't composite elements just for having an animated
            // 2D transform, so without this Safari repaints every node's
            // content on each pan/zoom frame, making large graphs very slow.
            "will-change-transform",
          ),
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

  // Place a new layout and fit the viewport to it in the same commit, so the
  // first painted frame is already fitted. Fitting a frame later (fitView
  // needs the nodes committed to the store first) would paint the whole graph
  // once at the stale viewport and then jump — an expensive wrong-layout flash
  // on large graphs, especially in Safari. The nodes' estimated sizes stand in
  // for measurements here; rows have fixed heights, so they agree.
  const placeAndFit = useCallback(
    (positions: NodePositions) => {
      const flowNodes = buildNodes(positions);
      // A focus-into-view request owns the viewport; don't fit over it.
      const fit = !pendingFocusRef.current;
      const { width, height } = storeApi.getState();
      if (fit && width && height) {
        // Move the viewport before committing the nodes so that the first
        // frame also culls against the fitted viewport (not just paints at
        // it).
        void setViewport(
          getViewportForBounds(
            getNodesBounds(flowNodes),
            width,
            height,
            INITIAL_FIT_VIEW.minZoom,
            MAX_ZOOM,
            INITIAL_FIT_VIEW.padding,
          ),
        );
      }
      setNodes(flowNodes);
      if (fit && (!width || !height)) {
        // The canvas hasn't been measured yet; fit once it has.
        window.requestAnimationFrame(() => {
          if (pendingFocusRef.current) return;
          void fitView(INITIAL_FIT_VIEW);
        });
      }
    },
    [buildNodes, setNodes, storeApi, setViewport, fitView],
  );

  // Re-run ELK whenever the graph changes, merging any saved manual layout.
  useEffect(() => {
    let cancelled = false;
    void computeElkLayout(graph, sizes).then(({ positions }) => {
      if (cancelled) return;
      placeAndFit(
        mergeSavedLayout(savedPositionsRef.current, positions, sizes),
      );
    });
    return () => {
      cancelled = true;
    };
  }, [graph, sizes, placeAndFit]);

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

  // Edges draw on canvas layers, not SVG: SVG path costs scale with each
  // path's extent in both WebKit and Gecko — hundreds of long-range reference
  // edges freeze Firefox below 1fps, and dragging a selected hub node
  // (reshaping all of its edges every frame) ran at ~2.5fps in Safari as SVG.
  // Active (selected/hovered) edges go on their own layer, drawn above.
  const { inactiveEdges, activeEdges } = useMemo(() => {
    const inactive: CanvasEdge[] = [];
    const active: CanvasEdge[] = [];
    graph.edges.forEach((edge) => {
      const hoverActive =
        hover?.kind === "field"
          ? edge.source === hover.table && edge.field === hover.field
          : hover?.kind === "header"
            ? edge.target === hover.table
            : false;
      const isActive =
        hoverActive ||
        (selectedTable !== null &&
          (edge.source === selectedTable || edge.target === selectedTable));
      (isActive ? active : inactive).push({
        id: edge.id,
        source: edge.source,
        target: edge.target,
      });
    });
    return { inactiveEdges: inactive, activeEdges: active };
  }, [graph.edges, hover, selectedTable]);

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
      placeAndFit(positions);
    });
  }, [clearSavedPositions, graph, sizes, placeAndFit]);

  // Cull offscreen elements on large graphs only.
  const cullOffscreen =
    graph.nodes.length + graph.edges.length > VISIBLE_CULL_THRESHOLD;

  return (
    <ReactFlow
      nodes={decoratedNodes}
      onlyRenderVisibleElements={cullOffscreen}
      nodeTypes={nodeTypes}
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
      minZoom={MIN_ZOOM}
      maxZoom={MAX_ZOOM}
      nodesDraggable={!isTouch}
      nodesConnectable={false}
      // Tab between tables; arrow keys move the focused one (React Flow built-in).
      nodesFocusable
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
      {/* After the background so they paint above the dots (all z-index -1,
          DOM order breaks the tie) and below the nodes; active above
          inactive. */}
      <SchemaEdge edges={inactiveEdges} appearance="inactive" />
      <SchemaEdge edges={activeEdges} appearance="active" />
      <SchemaSearch
        entries={searchEntries}
        onPick={onFocusTable}
        onOpenChange={setSearchOpen}
      />
      {!searchOpen && (
        <MinimapOverlay
          nodes={decoratedNodes}
          inactiveEdges={inactiveEdges}
          activeEdges={activeEdges}
        />
      )}
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
