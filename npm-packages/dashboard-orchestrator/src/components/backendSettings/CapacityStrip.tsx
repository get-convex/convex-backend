import { cn } from "@ui/cn";
import type { HostCapacity } from "../../lib/orchestratorApi";
import { lookupTier } from "./tiers";

export function CapacityStrip({
  capacity,
  selectedTier,
}: {
  capacity: HostCapacity | undefined;
  selectedTier: string;
}) {
  if (!capacity || capacity.totalMemoryMb === 0) {
    return null;
  }
  const selected = lookupTier(selectedTier);
  const projectedMb =
    capacity.allocatedMemoryMb + (selected?.memoryMb ?? 0);
  const projectedPct = Math.min(
    100,
    Math.round((projectedMb / capacity.totalMemoryMb) * 100),
  );
  const currentPct = Math.round(
    (capacity.allocatedMemoryMb / capacity.totalMemoryMb) * 100,
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
          Host capacity {(capacity.allocatedMemoryMb / 1024).toFixed(1)} /{" "}
          {(capacity.totalMemoryMb / 1024).toFixed(1)} GB allocated
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
    </div>
  );
}
