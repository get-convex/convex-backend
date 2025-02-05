import { HealthCard } from "@common/elements/HealthCard";
import { useTopKFunctionMetrics } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";

export function FailureRate() {
  const chartData = useTopKFunctionMetrics("failurePercentage");

  return (
    <HealthCard
      title="Failure Rate"
      tip="The failure rate of all your running functions, bucketed by minute."
    >
      <ChartForFunctionRate chartData={chartData} kind="failureRate" />
    </HealthCard>
  );
}
