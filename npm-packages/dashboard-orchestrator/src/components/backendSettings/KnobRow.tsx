import { Button } from "@ui/Button";
import { cn } from "@ui/cn";
import type { KnobEntry } from "../../lib/orchestratorApi";
import { KnobInput } from "./KnobInput";

type Source = "default" | "tier" | "override";

export function KnobRow({
  knob,
  source,
  effectiveValue,
  overrideValue,
  onOverride,
  onReset,
}: {
  knob: KnobEntry;
  source: Source;
  effectiveValue: string;
  overrideValue: string;
  onOverride: (next: string) => void;
  onReset: () => void;
}) {
  const overridden = source === "override";
  return (
    <div className="flex flex-col gap-1 py-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <div className="flex flex-col">
          <span className="text-sm font-medium text-content-primary">
            {knob.displayName ?? knob.envVar}
          </span>
          <span className="font-mono text-xs text-content-secondary">
            {knob.envVar}
          </span>
        </div>
        <SourcePill source={source} />
      </div>
      {knob.description && (
        <p className="max-w-prose text-xs text-content-secondary">
          {knob.description}
        </p>
      )}
      <div className="flex items-center gap-2">
        <div className="grow">
          <KnobInput
            knob={knob}
            value={overridden ? overrideValue : effectiveValue}
            onChange={onOverride}
          />
        </div>
        {overridden && (
          <Button variant="neutral" size="xs" onClick={onReset}>
            Revert to default
          </Button>
        )}
      </div>
    </div>
  );
}

function SourcePill({ source }: { source: Source }) {
  const label =
    source === "override"
      ? "Override"
      : source === "tier"
        ? "Tier default"
        : "Default";
  return (
    <span
      className={cn(
        "rounded-full px-2 py-0.5 text-[10px] font-medium uppercase",
        source === "override"
          ? "bg-background-highlight text-content-accent"
          : "bg-background-tertiary text-content-secondary",
      )}
    >
      {label}
    </span>
  );
}
