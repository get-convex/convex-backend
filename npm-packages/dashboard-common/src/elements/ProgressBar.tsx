import { cn } from "@common/lib/cn";

export function ProgressBar({
  fraction,
  ariaLabel,
  variant = "stripes",
  className,
}: {
  fraction: undefined | number;
  ariaLabel: string;
  variant?: "stripes" | "solid";
  className?: string;
}) {
  return (
    <div
      className={cn(
        "h-4 overflow-hidden rounded bg-background-tertiary",
        className,
      )}
      role="progressbar"
      aria-valuenow={fraction !== undefined ? fraction * 100 : undefined}
      aria-label={ariaLabel}
    >
      <div
        className="h-full w-full overflow-hidden motion-safe:transition-[clip-path]"
        style={{
          clipPath:
            fraction !== undefined
              ? `polygon(0 0, ${fraction * 100}% 0, ${fraction * 100}% 100%, 0 100%)`
              : undefined,
        }}
      >
        <div
          className={cn("box-content h-full w-full bg-util-accent pl-8", {
            "motion-safe:animate-[.5s_indeterminateProgressBar_infinite_linear]":
              variant === "stripes",
          })}
          style={{
            backgroundImage:
              variant === "stripes"
                ? "linear-gradient(45deg, rgba(255, 255, 255, 0.15) 25%, rgba(0, 0, 0, 0) 25%, rgba(0, 0, 0, 0) 50%, rgba(255, 255, 255, 0.15) 50%, rgba(255, 255, 255, 0.15) 75%, rgba(0, 0, 0, 0) 75%, rgba(0, 0, 0, 0))"
                : undefined,
            backgroundSize: "1rem",
          }}
        />
      </div>
    </div>
  );
}
