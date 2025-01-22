import { cn } from "lib/cn";
import { ReactNode } from "react";

export type MetricHealth = "healthy" | "warning" | "error";

export function BigMetric({
  health,
  metric,
  children,
}: {
  health?: MetricHealth;
  metric: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex h-52 animate-fadeInFromLoading flex-col items-center justify-center gap-2 px-2 pb-2">
      {/* eslint-disable-next-line no-restricted-syntax */}
      <div
        className={cn(
          // eslint-disable-next-line no-restricted-syntax
          "text-4xl font-semibold",
          {
            "text-content-success": health === "healthy",
            "text-content-warning": health === "warning",
            "text-content-error": health === "error",
          },
        )}
      >
        {metric}
      </div>
      <div className="max-h-10 min-h-10 truncate text-pretty text-center text-content-secondary">
        {children}
      </div>
    </div>
  );
}
