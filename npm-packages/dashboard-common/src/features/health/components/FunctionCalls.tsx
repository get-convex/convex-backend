import { HealthCard } from "@common/elements/HealthCard";
import { useFunctionCallCountTopK } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";

export function FunctionCalls() {
  const chartData = useFunctionCallCountTopK(5);

  return (
    <HealthCard
      title="Function Calls"
      tip="The most frequently called functions in this deployment, bucketed by minute."
    >
      <ChartForFunctionRate chartData={chartData} kind="functionCalls" />
    </HealthCard>
  );
}
