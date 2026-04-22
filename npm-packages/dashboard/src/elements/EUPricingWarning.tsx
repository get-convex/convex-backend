import { cn } from "@ui/cn";

export function EUPricingWarning({ show }: { show: boolean }) {
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
