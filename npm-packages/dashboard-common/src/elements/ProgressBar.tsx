export function ProgressBar({
  fraction,
  ariaLabel,
}: {
  fraction: undefined | number;
  ariaLabel: string;
}) {
  return (
    <div
      className="h-4 overflow-hidden rounded bg-background-tertiary"
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
          className="box-content h-full w-full bg-util-accent pl-8 motion-safe:animate-[.5s_indeterminateProgressBar_infinite_linear]"
          style={{
            backgroundImage:
              "linear-gradient(45deg, rgba(255, 255, 255, 0.15) 25%, rgba(0, 0, 0, 0) 25%, rgba(0, 0, 0, 0) 50%, rgba(255, 255, 255, 0.15) 50%, rgba(255, 255, 255, 0.15) 75%, rgba(0, 0, 0, 0) 75%, rgba(0, 0, 0, 0))",
            backgroundSize: "1rem",
          }}
        />
      </div>
    </div>
  );
}
