import { formatDistance } from "date-fns";
import { cn } from "@ui/cn";
import { HealthCard } from "@common/elements/HealthCard";
import { useSchedulerLag } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";
import { ChartData } from "@common/lib/charts/types";

export function SchedulerStatus({
  small = false,
  lag: lagProp,
}: {
  small?: boolean;
  lag?: ChartData | null | undefined;
}) {
  const lagFromHook = useSchedulerLag();
  const lag = lagProp ?? lagFromHook;
  const lagData = lag?.data as Array<{ time: string; lag: number }> | undefined;
  const behindBySeconds =
    60 * ((lagData && lagData[lagData.length - 1]?.lag) || 0);

  const health =
    behindBySeconds <= 20
      ? "healthy"
      : behindBySeconds > 300
        ? "error"
        : "warning";

  if (small) {
    if (!health || health === "healthy") {
      return null;
    }
    return (
      <div className="flex animate-fadeInFromLoading flex-col place-content-center items-center justify-center text-xs">
        <p
          className={cn("font-semibold", {
            "text-content-warning": health === "warning",
            "text-content-error": health === "error",
          })}
        >
          Overdue
        </p>
        <div className="truncate text-center text-pretty text-content-secondary">
          <div className="flex gap-1">
            <p className="text-content-secondary">
              Scheduling is behind by{" "}
              {formatDistance(0, behindBySeconds * 1000)}.
            </p>
          </div>
        </div>
      </div>
    );
  }
  return (
    <HealthCard
      title="Scheduler Status"
      tip="The status of function scheduling. Scheduling is unhealthy when functions are executing after their scheduled time."
    >
      <ChartForFunctionRate chartData={lag} kind="schedulerStatus" />
    </HealthCard>
  );
}
