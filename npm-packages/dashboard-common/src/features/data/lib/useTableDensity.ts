export function useTableDensity() {
  const dv = densityValues.normal;
  return {
    densityValues: {
      ...dv,
      height: dv.lineHeight + 2 * (dv.paddingY + dv.border),
    },
  };
}

const densityValues: Record<
  "normal",
  { paddingX: number; paddingY: number; lineHeight: number; border: number }
> = {
  // Even tighter values we could use in the future.
  // snug: { paddingX: 6, paddingY: 8, lineHeight: 16, border: 1 },
  normal: { paddingX: 12, paddingY: 12, lineHeight: 16, border: 1 },
  // Old density values.
  // comfy: { paddingX: 12, paddingY: 16, lineHeight: 16, border: 1 },
};
