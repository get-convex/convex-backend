import { cn } from "@ui/cn";
import type { HostCapacity } from "../../lib/orchestratorApi";
import { formatTierResources, tierResourcesForName } from "./tiers";

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
  const current = currentTier ? tierResourcesForName(currentTier) : undefined;
  // Subtract the existing deployment's own slice (if any) from the host
  // total so a resize doesn't compare the new tier against its own
  // allocation. Legacy `max` rows are shown as consuming the whole host.
  const currentMemoryMb =
    current?.memoryMb ?? (currentTier === "max" ? capacity.totalMemoryMb : 0);
  const baselineAllocatedMb = Math.max(
    0,
    capacity.allocatedMemoryMb - currentMemoryMb,
  );

  const projectedMb = baselineAllocatedMb + (selected?.memoryMb ?? 0);
  const projectedPct = Math.round((projectedMb / capacity.totalMemoryMb) * 100);
  const projectedBarPct = Math.min(100, projectedPct);
  const currentPct = Math.min(
    100,
    Math.round((baselineAllocatedMb / capacity.totalMemoryMb) * 100),
  );
  const over = projectedMb > capacity.totalMemoryMb;
  const warn = !over && projectedBarPct >= 80;
  /* eslint-disable no-restricted-syntax -- progress bar fill; bg-content-* is appropriate for small indicator fills */
  const fillColor = over
    ? "bg-content-warning"
    : warn
      ? "bg-content-warning"
      : "bg-content-success";
  /* eslint-enable no-restricted-syntax */

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-xs text-content-secondary">
        <span>
          Host allocation {(baselineAllocatedMb / 1024).toFixed(1)} /{" "}
          {(capacity.totalMemoryMb / 1024).toFixed(1)} GB allocated
          {currentTier ? " (excluding this deployment)" : ""}
        </span>
        <span className="font-mono">{currentPct}%</span>
      </div>
      <div className="relative h-2 w-full overflow-hidden rounded-full bg-background-tertiary">
        <div
          className={cn("absolute inset-y-0 left-0 transition-all", fillColor)}
          style={{ width: `${projectedBarPct}%` }}
        />
        {selected && (
          <div
            className="absolute inset-y-0 bg-current opacity-40"
            style={{
              left: `${currentPct}%`,
              width: `${Math.max(0, projectedBarPct - currentPct)}%`,
            }}
            aria-label="Projected delta"
          />
        )}
      </div>
      {(warn || over) && (
        <p className={cn("text-xs", "text-content-warning")}>
          {over
            ? `${selectedTier} would push allocation to ${projectedPct}% (${formatTierResources(
                selected ?? { memoryMb: 0, cpus: 0 },
              )}); overprovisioning is allowed.`
            : `${selectedTier} would push allocation to ${projectedPct}% — close to host capacity.`}
        </p>
      )}
    </div>
  );
}
