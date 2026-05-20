import { cn } from "@ui/cn";
import type { HostCapacity } from "../../lib/orchestratorApi";
import { TIERS } from "./tiers";

export function TierSelector({
  value,
  onChange,
  capacity,
}: {
  value: string;
  onChange: (tier: string) => void;
  capacity: HostCapacity | undefined;
}) {
  return (
    <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
      {TIERS.map((tier) => {
        // Unbounded tiers project as consuming the whole host; treat as
        // total+1 so the "would exceed" check fires unless the host is empty.
        const projectedMb = tier.unbounded
          ? (capacity?.totalMemoryMb ?? 0) + 1
          : (capacity?.allocatedMemoryMb ?? 0) + tier.memoryMb;
        const wouldExceed =
          capacity !== undefined && projectedMb > capacity.totalMemoryMb;
        const selected = value === tier.name;
        return (
          // eslint-disable-next-line react/forbid-elements -- card-style radio, intentional plain button
          <button
            key={tier.name}
            type="button"
            disabled={wouldExceed}
            onClick={() => onChange(tier.name)}
            aria-pressed={selected}
            className={cn(
              "flex flex-col items-start rounded-md border px-3 py-2 text-left transition-all",
              selected
                ? "border-content-link bg-background-secondary"
                : "border-border-transparent bg-background-tertiary/40 hover:bg-background-tertiary",
              wouldExceed && "cursor-not-allowed opacity-50",
            )}
            title={
              wouldExceed
                ? tier.unbounded
                  ? "max requires an empty host — all capacity would be consumed."
                  : `Would exceed host capacity (need ${tier.memoryMb} MB).`
                : undefined
            }
          >
            <span className="font-mono text-sm font-semibold">{tier.name}</span>
            <span className="text-xs text-content-secondary">
              {tier.unbounded
                ? "Unbounded"
                : `${(tier.memoryMb / 1024).toFixed(0)} GB · ${tier.cpus} CPUs`}
            </span>
          </button>
        );
      })}
    </div>
  );
}
