import { type ElkNode } from "elkjs/lib/elk.bundled.js";
import { SchemaGraph } from "@common/features/schema/lib/buildSchemaGraph";
import {
  NodePositions,
  NodeSize,
  NODE_WIDTH,
} from "@common/features/schema/lib/layout";

type ElkInstance = { layout: (graph: ElkNode) => Promise<ElkNode> };
let elkPromise: Promise<ElkInstance> | undefined;
function getElk(): Promise<ElkInstance> {
  if (!elkPromise) {
    elkPromise = import("elkjs/lib/elk.bundled.js").then(
      (mod) => new mod.default() as ElkInstance,
    );
  }
  return elkPromise;
}

const LAYOUT_OPTIONS = {
  "elk.algorithm": "layered",
  "elk.layered.nodePlacement.favorStraightEdges": "true",
  "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
  "elk.direction": "DOWN",
  "elk.spacing.nodeNode": "64",
  "elk.spacing.edgeNode": "24",
  "elk.layered.spacing.nodeNodeBetweenLayers": "120",
  "elk.spacing.edgeEdge": "10",
  // Pack disconnected tables (each its own component) close together instead
  // of scattering them across the canvas.
  "elk.spacing.componentComponent": "64",
};

export async function computeElkLayout(
  graph: SchemaGraph,
  sizes: Record<string, NodeSize>,
): Promise<{ positions: NodePositions }> {
  if (graph.nodes.length === 0) {
    return { positions: {} };
  }

  const elkGraph: ElkNode = {
    id: "root",
    layoutOptions: LAYOUT_OPTIONS,
    children: graph.nodes.map((node) => ({
      id: node.table,
      width: sizes[node.table]?.width ?? NODE_WIDTH,
      height: sizes[node.table]?.height ?? 100,
    })),
    // ELK can't route a self-loop the same way; skip those (React Flow draws
    // its own self-loop edge).
    edges: graph.edges
      .filter((edge) => edge.source !== edge.target)
      .map((edge) => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
      })),
  };

  const elk = await getElk();
  const laidOut = await elk.layout(elkGraph);

  const positions: NodePositions = {};
  (laidOut.children ?? []).forEach((child) => {
    positions[child.id] = { x: child.x ?? 0, y: child.y ?? 0 };
  });

  return { positions };
}
