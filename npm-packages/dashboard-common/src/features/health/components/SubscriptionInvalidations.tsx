import { HealthCard } from "@common/elements/HealthCard";
import { useSubscriptionInvalidationsTopK } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";

export function SubscriptionInvalidations() {
  const chartData = useSubscriptionInvalidationsTopK(5);

  return (
    <HealthCard
      title="Subscription Invalidations"
      tip="The mutation and table pairs that invalidate the most subscriptions, bucketed by minute."
    >
      <ChartForFunctionRate
        chartData={chartData}
        kind="subscriptionInvalidations"
      />
    </HealthCard>
  );
}
