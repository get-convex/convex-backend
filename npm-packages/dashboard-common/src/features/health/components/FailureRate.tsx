import { HealthCard } from "@common/elements/HealthCard";
import { useTopKFunctionMetrics } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";
import type { ChartData } from "@common/lib/charts/types";

export function FailureRateCard({
  chartData,
}: {
  chartData: ChartData | null | undefined;
}) {
  return (
    <HealthCard
      title="Failure Rate"
      tip="The failure rate of all your running functions, bucketed by minute."
    >
      <ChartForFunctionRate chartData={chartData} kind="failureRate" />
    </HealthCard>
  );
}

export function FailureRate() {
  const chartData = useTopKFunctionMetrics("failurePercentage");
  return <FailureRateCard chartData={chartData} />;
}
