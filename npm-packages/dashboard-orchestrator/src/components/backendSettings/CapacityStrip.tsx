import { cn } from "@ui/cn";
import type { HostCapacity } from "../../lib/orchestratorApi";
import { lookupTier } from "./tiers";

export function CapacityStrip({
  capacity,
  selectedTier,
  currentTier,
  force,
  onForceChange,
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
  force?: boolean;
  onForceChange?: (force: boolean) => void;
}) {
  if (!capacity || capacity.totalMemoryMb === 0) {
    return null;
  }
  const selected = lookupTier(selectedTier);
  const current = currentTier ? lookupTier(currentTier) : undefined;
  // Subtract the existing deployment's own slice (if any) from the host
  // total so a resize doesn't compare the new tier against its own
  // allocation. Unbounded current tiers consume the whole host — pretend
  // they don't, since they're being replaced.
  const baselineAllocatedMb =
    current && !current.unbounded
      ? Math.max(0, capacity.allocatedMemoryMb - current.memoryMb)
      : current?.unbounded
        ? 0
        : capacity.allocatedMemoryMb;

  // Unbounded tier consumes all host capacity.
  if (selected?.unbounded) {
    const currentPct = Math.round(
      (baselineAllocatedMb / capacity.totalMemoryMb) * 100,
    );
    const isOver = baselineAllocatedMb > 0;
    /* eslint-disable no-restricted-syntax -- progress bar fill; bg-content-* is appropriate for small indicator fills */
    const fillColor = isOver ? "bg-content-error" : "bg-content-warning";
    /* eslint-enable no-restricted-syntax */
    return (
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between text-xs text-content-secondary">
          <span>
            Host capacity {(baselineAllocatedMb / 1024).toFixed(1)} /{" "}
            {(capacity.totalMemoryMb / 1024).toFixed(1)} GB allocated
            {currentTier ? " (excluding this deployment)" : ""}
          </span>
          <span className="font-mono">{currentPct}%</span>
        </div>
        <div className="relative h-2 w-full overflow-hidden rounded-full bg-background-tertiary">
          <div
            className={cn(
              "absolute inset-y-0 left-0 transition-all",
              fillColor,
            )}
            style={{ width: "100%" }}
          />
        </div>
        <p
          className={cn(
            "text-xs",
            isOver ? "text-content-error" : "text-content-warning",
          )}
        >
          max consumes all host capacity — no further deployments can be
          provisioned
        </p>
      </div>
    );
  }

  const projectedMb = baselineAllocatedMb + (selected?.memoryMb ?? 0);
  const projectedPct = Math.min(
    100,
    Math.round((projectedMb / capacity.totalMemoryMb) * 100),
  );
  const currentPct = Math.round(
    (baselineAllocatedMb / capacity.totalMemoryMb) * 100,
  );
  const over = projectedMb > capacity.totalMemoryMb;
  const warn = !over && projectedPct >= 80;
  /* eslint-disable no-restricted-syntax -- progress bar fill; bg-content-* is appropriate for small indicator fills */
  const fillColor = over
    ? "bg-content-error"
    : warn
      ? "bg-content-warning"
      : "bg-content-success";
  /* eslint-enable no-restricted-syntax */

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-xs text-content-secondary">
        <span>
          Host capacity {(baselineAllocatedMb / 1024).toFixed(1)} /{" "}
          {(capacity.totalMemoryMb / 1024).toFixed(1)} GB allocated
          {currentTier ? " (excluding this deployment)" : ""}
        </span>
        <span className="font-mono">{currentPct}%</span>
      </div>
      <div className="relative h-2 w-full overflow-hidden rounded-full bg-background-tertiary">
        <div
          className={cn("absolute inset-y-0 left-0 transition-all", fillColor)}
          style={{ width: `${projectedPct}%` }}
        />
        {selected && (
          <div
            className="absolute inset-y-0 bg-current opacity-40"
            style={{
              left: `${currentPct}%`,
              width: `${projectedPct - currentPct}%`,
            }}
            aria-label="Projected delta"
          />
        )}
      </div>
      {(warn || over) && (
        <p
          className={cn(
            "text-xs",
            over ? "text-content-error" : "text-content-warning",
          )}
        >
          {over
            ? `${selectedTier} would push to ${projectedPct}% — exceeds host capacity. Provisioning will fail.`
            : `${selectedTier} would push to ${projectedPct}% — close to host capacity.`}
        </p>
      )}
      {over && onForceChange !== undefined && (
        <label className="flex cursor-pointer items-center gap-2 text-xs text-content-secondary">
          {/* eslint-disable-next-line react/forbid-elements -- plain checkbox for force-provision opt-in */}
          <input
            type="checkbox"
            checked={force ?? false}
            onChange={(e) => onForceChange(e.target.checked)}
            className="h-3 w-3"
          />
          Force provision (over-commit this host)
        </label>
      )}
    </div>
  );
}
