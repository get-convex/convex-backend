import { formatDistance } from "date-fns";
import { useState } from "react";
import { BarChartIcon, ResetIcon } from "@radix-ui/react-icons";
import { cn } from "lib/cn";
import { useLogDeploymentEvent, Button } from "dashboard-common";
import { HealthCard } from "./HealthCard";
import { useSchedulerLag } from "../lib/appMetrics";
import { ChartForFunctionRate } from "../features/health/components/ChartForFunctionRate";
import { BigMetric } from "./BigMetric";

export function SchedulerStatus({ small = false }: { small?: boolean }) {
  const lag = useSchedulerLag();
  const behindBySeconds =
    60 * ((lag && lag.data[lag.data.length - 1].lag) || 0);

  const log = useLogDeploymentEvent();

  const health =
    behindBySeconds <= 5
      ? "healthy"
      : behindBySeconds > 300
        ? "error"
        : "warning";

  const [showChart, setShowChart] = useState(false);

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
        <div className="truncate text-pretty text-center text-content-secondary">
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
      action={
        <Button
          size="xs"
          variant="neutral"
          onClick={() => {
            setShowChart(!showChart);
            log("toggle scheduler chart", { showChart: !showChart });
          }}
          icon={showChart ? <ResetIcon /> : <BarChartIcon />}
          tip={showChart ? "Hide Chart" : "Show Chart"}
          inline
        />
      }
    >
      {showChart && (
        <ChartForFunctionRate chartData={lag} kind="schedulerStatus" />
      )}
      {!showChart &&
        (health !== "healthy" ? (
          <BigMetric metric="Overdue" health="error">
            Scheduling is behind by {formatDistance(0, behindBySeconds * 1000)}.
          </BigMetric>
        ) : (
          <BigMetric metric="On time">
            <p className="min-h-10 text-pretty text-center text-content-secondary">
              Scheduled functions are running on time.
            </p>
          </BigMetric>
        ))}
    </HealthCard>
  );
}
