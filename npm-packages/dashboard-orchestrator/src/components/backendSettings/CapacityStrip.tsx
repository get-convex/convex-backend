import { cn } from "@ui/cn";
import type { HostCapacity } from "../../lib/orchestratorApi";
import { formatTierResources, tierResourcesForName } from "./tiers";

export type CapacityProjection = {
  baselineAllocatedMb: number;
  projectedAllocatedMb: number;
  baselinePct: number;
  projectedPct: number;
  projectedBarPct: number;
  over: boolean;
  warn: boolean;
};

export function capacityProjection(
  capacity: HostCapacity,
  selectedTier: string,
  currentTier?: string,
): CapacityProjection {
  const selected = tierResourcesForName(selectedTier);
  const current = currentTier ? tierResourcesForName(currentTier) : undefined;
  const currentMemoryMb =
    current?.memoryMb ?? (currentTier === "max" ? capacity.totalMemoryMb : 0);
  const baselineAllocatedMb = Math.max(
    0,
    capacity.allocatedMemoryMb - currentMemoryMb,
  );
  const projectedAllocatedMb = baselineAllocatedMb + (selected?.memoryMb ?? 0);
  const projectedPct = Math.round(
    (projectedAllocatedMb / capacity.totalMemoryMb) * 100,
  );
  const projectedBarPct = Math.min(100, projectedPct);
  const baselinePct = Math.min(
    100,
    Math.round((baselineAllocatedMb / capacity.totalMemoryMb) * 100),
  );
  const over = projectedAllocatedMb > capacity.totalMemoryMb;
  const warn = !over && projectedBarPct >= 80;
  return {
    baselineAllocatedMb,
    projectedAllocatedMb,
    baselinePct,
    projectedPct,
    projectedBarPct,
    over,
    warn,
  };
}

export function CapacityStrip({
  capacity,
  selectedTier,
  currentTier,
}: {
  capacity: HostCapacity | undefined;
  selectedTier: string;
  /**
   * When set, the strip subtracts this tier's memory from
   * `capacity.allocatedMemoryMb` before projecting the selected tier.
   * Used on the per-deployment settings page so resizing an existing
   * deployment doesn't double-count its own current allocation.
   * Leave undefined for the project-creation flow (new deployment).
   */
  currentTier?: string;
}) {
  if (!capacity || capacity.totalMemoryMb === 0) {
    return null;
  }
  const selected = tierResourcesForName(selectedTier);
  const projection = capacityProjection(capacity, selectedTier, currentTier);
  const projectedGb = (projection.projectedAllocatedMb / 1024).toFixed(1);
  const totalGb = (capacity.totalMemoryMb / 1024).toFixed(1);
  const baselineGb = (projection.baselineAllocatedMb / 1024).toFixed(1);
  /* eslint-disable no-restricted-syntax -- progress bar fill; bg-content-* is appropriate for small indicator fills */
  const fillColor = projection.over
    ? "bg-content-warning"
    : projection.warn
      ? "bg-content-warning"
      : "bg-content-success";
  /* eslint-enable no-restricted-syntax */
  const deltaWidth = Math.max(
    0,
    projection.projectedBarPct - projection.baselinePct,
  );

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-xs text-content-secondary">
        <span>
          Host allocation after save {projectedGb} / {totalGb} GB
        </span>
        <span className="font-mono">{projection.projectedPct}%</span>
      </div>
      <div
        className="relative h-2 w-full overflow-hidden rounded-full bg-background-tertiary"
        aria-label="Projected host allocation"
      >
        <div
          className={cn("absolute inset-y-0 left-0 transition-all", fillColor)}
          style={{ width: `${projection.projectedBarPct}%` }}
        />
        {selected && deltaWidth > 0 && (
          <div
            className="absolute inset-y-0 bg-current opacity-40"
            style={{
              left: `${projection.baselinePct}%`,
              width: `${deltaWidth}%`,
            }}
            aria-label="Projected delta"
          />
        )}
      </div>
      {currentTier && (
        <div className="text-[11px] text-content-secondary">
          Other deployments: {baselineGb} GB allocated
        </div>
      )}
      {(projection.warn || projection.over) && (
        <p className={cn("text-xs", "text-content-warning")}>
          {projection.over
            ? `${selectedTier} would push allocation to ${projection.projectedPct}% (${formatTierResources(
                selected ?? { memoryMb: 0, cpus: 0 },
              )}); overprovisioning is allowed.`
            : `${selectedTier} would push allocation to ${projection.projectedPct}% - close to host capacity.`}
        </p>
      )}
    </div>
  );
}
