import { HealthCard } from "elements/HealthCard";
import { useTopKFunctionMetrics } from "lib/appMetrics";
import { ChartForFunctionRate } from "features/health/components/ChartForFunctionRate";

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
