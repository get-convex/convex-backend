import { type Node, type Edge } from "@xyflow/react";
import { SchemaNode } from "@common/features/schema/lib/buildSchemaGraph";

// What the cursor is over: a table's header (emphasizes the references pointing
// *to* it), or a specific field/index row (highlights that row, plus — for a
// field — the reference it points *out* to).
export type HoverTarget =
  | { kind: "header"; table: string }
  | { kind: "field"; table: string; field: string }
  | { kind: "index"; table: string; index: string };

export type TableNodeData = {
  node: SchemaNode;
  isSelected: boolean;
  // Field rows to emphasize (the hovered field, or fields pointing at a
  // hovered/selected table).
  highlightedFields: ReadonlySet<string>;
  // Index rows to emphasize (the hovered index).
  highlightedIndexes: ReadonlySet<string>;
  // Report what the cursor is over (null when it leaves the node).
  onHover: (target: HoverTarget | null) => void;
};
export type TableFlowNode = Node<TableNodeData, "table">;

// Set on edges touching the hovered/selected table; keeps their arrowheads
// visible when zoomed out.
export type ElkEdgeData = { active: boolean };
export type ElkFlowEdge = Edge<ElkEdgeData, "elk">;
