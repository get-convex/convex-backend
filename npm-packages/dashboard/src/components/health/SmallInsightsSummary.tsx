import { useCurrentDeployment } from "api/deployments";
import { useInsightsPeriod, useInsightsSummary } from "api/insights";
import { Button } from "@ui/Button";
import { HealthCard } from "@common/elements/HealthCard";
import { ChevronRightIcon } from "@radix-ui/react-icons";
import { InsightsSummary } from "./InsightsSummary";

export function SmallInsightsSummary({ onViewAll }: { onViewAll: () => void }) {
  const deployment = useCurrentDeployment();
  const insights = useInsightsSummary();
  const { from } = useInsightsPeriod();

  if (deployment?.kind === "local") {
    return null;
  }

  return (
    <div>
      <HealthCard
        title="Insights"
        tip="Get proactive insight into your function health."
        size="lg"
        action={
          <span className="text-xs text-content-secondary">
            {new Date(from).toLocaleString([], {
              year: "numeric",
              month: "numeric",
              day: "numeric",
              hour: "numeric",
              minute: undefined,
            })}{" "}
            – Now
          </span>
        }
      >
        <InsightsSummary insights={insights?.slice(0, 5)} />
      </HealthCard>
      <div className="flex">
        {insights && insights.length > 0 && (
          <Button
            variant="neutral"
            className="m-auto mt-2 w-fit gap-1 hover:bg-background-tertiary"
            onClick={onViewAll}
          >
            View{" "}
            {insights.length > 6
              ? `${insights.length - 5} more Insights`
              : "all Insights"}{" "}
            <ChevronRightIcon />
          </Button>
        )}
      </div>
    </div>
  );
}
