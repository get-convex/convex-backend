import { cn } from "@ui/cn";
import type { RegionName } from "generatedApi";
import { PRIMARY_REGION } from "lib/regions";

/**
 * Warns that a region is billed differently from the primary one. Always
 * rendered so that picking a region doesn't shift the surrounding layout.
 */
export function RegionPricingWarning({
  region,
}: {
  region: RegionName | null;
}) {
  const show = region !== null && region !== PRIMARY_REGION;

  return (
    <p
      className={cn(
        "mt-2 text-xs text-content-warning transition-opacity",
        show ? "opacity-100" : "opacity-0 select-none",
        // `relative` fixes a weird browser bug where Safari would sometimes ignore opacity-0
        "relative",
      )}
      inert={!show}
      aria-hidden={!show}
    >
      No included limits (all usage billed on-demand) + 30% regional surcharge
    </p>
  );
}
