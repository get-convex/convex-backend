import {
  BaseEdge,
  useStore,
  type EdgeProps,
  type EdgeTypes,
} from "@xyflow/react";
import { NODE_WIDTH, zoomStep } from "@common/features/schema/lib/layout";
import { ElkFlowEdge } from "@common/features/schema/components/schemaFlowTypes";

const ARROWHEAD_HIDE_LEVEL = 2;

function ElkEdge({
  source,
  target,
  sourceX,
  sourceY,
  targetX,
  targetY,
  style,
  data,
}: EdgeProps<ElkFlowEdge>) {
  const zoomLevel = useStore((s) => zoomStep(s.transform[2]));
  // On-screen stroke weight is strokeWidth * zoom, so edges thin out when zoomed
  // out. Compensate with the quantized zoom factor to keep a steady weight.
  const baseWidth =
    typeof style?.strokeWidth === "number" ? style.strokeWidth : 1.5;
  const strokeWidth = baseWidth * 2 ** zoomLevel;
  const headLength = strokeWidth * 6;
  const headHalf = strokeWidth * 3;

  // Inline filled-triangle arrowhead, sized off the stroke width. Dropped on
  // non-active edges once zoomed out far enough that it's just noise.
  const showArrowhead = data?.active || zoomLevel < ARROWHEAD_HIDE_LEVEL;

  let path: string;
  // Arrowhead tip and travel direction. Drawn inline (not an SVG marker) because
  // Safari intermittently fails to repaint url(#marker) across viewport transforms.
  let tipX: number;
  let tipY: number;
  let dirX: number;
  let dirY: number;
  if (source === target) {
    // Self-reference: one symmetric loop bulging off the node's right edge (both
    // handles sit at the node's horizontal center, so the right edge is at
    // sourceX + NODE_WIDTH / 2). Two control points out to the right, mirrored
    // above/below center, draw it as a single cubic. The opaque arrowhead covers
    // the line end, so — unlike the other branch — no trimming is needed.
    const edgeX = sourceX + NODE_WIDTH / 2;
    const cy = (sourceY + targetY) / 2;
    const gap = 7; // where the loop's two ends meet the edge, above/below center
    const outX = 40; // how far the loop reaches to the right
    const outY = 18; // half the loop's height
    tipX = edgeX;
    tipY = cy + gap;
    // Aim the arrowhead along the loop's incoming tangent (lower control point → tip).
    const dx = tipX - (edgeX + outX);
    const dy = tipY - (cy + outY);
    const len = Math.hypot(dx, dy) || 1;
    dirX = dx / len;
    dirY = dy / len;
    path =
      `M ${edgeX},${cy - gap} ` +
      `C ${edgeX + outX},${cy - outY} ${edgeX + outX},${cy + outY} ${edgeX},${cy + gap}`;
  } else {
    // Cubic bezier from the source's bottom handle into the target's top,
    // matching React Flow's getBezierPath (curvature 0.25). The trailing control
    // point sits directly above the target, so the curve always enters the top
    // handle vertically — the arrowhead points straight down.
    const curvature = 0.25;
    const span = targetY - sourceY;
    const offset = span >= 0 ? 0.5 * span : curvature * 25 * Math.sqrt(-span);
    const c1y = sourceY + offset;
    const c2y = targetY - offset;
    tipX = targetX;
    tipY = targetY;
    dirX = 0;
    dirY = 1;
    path = `M ${sourceX},${sourceY} C ${sourceX},${c1y} ${targetX},${c2y} ${targetX},${targetY}`;
  }

  const color =
    typeof style?.stroke === "string" ? style.stroke : "currentColor";
  const baseX = tipX - dirX * headLength;
  const baseY = tipY - dirY * headLength;
  const perpX = -dirY;
  const perpY = dirX;
  const arrowPoints = `${tipX},${tipY} ${baseX + perpX * headHalf},${
    baseY + perpY * headHalf
  } ${baseX - perpX * headHalf},${baseY - perpY * headHalf}`;

  return (
    <>
      <BaseEdge
        path={path}
        interactionWidth={0}
        style={{ ...style, strokeWidth, fill: "none", pointerEvents: "none" }}
      />
      {showArrowhead && (
        <polygon points={arrowPoints} fill={color} pointerEvents="none" />
      )}
    </>
  );
}

export const edgeTypes: EdgeTypes = { elk: ElkEdge };
