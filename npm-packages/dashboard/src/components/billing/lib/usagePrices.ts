type PricingPeriod = {
  start: number;
  end: number | null;
};

export function getUsagePricingPeriod(
  prices: { periods: PricingPeriod[] } | undefined,
  range: { from: string; to: string } | null,
): PricingPeriod | undefined {
  if (!prices || !range) return undefined;
  const start = Date.parse(range.from);
  const end = Date.parse(range.to);
  if (!Number.isFinite(start) || !Number.isFinite(end) || start > end) {
    return undefined;
  }
  return prices.periods.find(
    (period) =>
      start >= period.start && (period.end === null || end <= period.end),
  );
}
