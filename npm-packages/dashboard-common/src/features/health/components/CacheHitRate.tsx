import { HealthCard } from "@common/elements/HealthCard";
import { useTopKFunctionMetrics } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";

export function CacheHitRate() {
  const chartData = useTopKFunctionMetrics("cacheHitPercentage");

  return (
    <HealthCard
      title="Cache Hit Rate"
      tip="The cache hit rate of all your running query functions, bucketed by minute."
    >
      <ChartForFunctionRate chartData={chartData} kind="cacheHitRate" />
    </HealthCard>
  );
}
