import { useCurrentDeployment } from "api/deployments";
import { useCurrentTeam } from "api/teams";
import { useTeamOrbSubscription } from "api/billing";
import {
  UPSELL_INSIGHTS,
  useInsightsPeriod,
  useInsightsSummary,
} from "api/insights";
import {
  Button,
  Loading,
  useLogDeploymentEvent,
  Sheet,
  HealthCard,
} from "dashboard-common";
import {
  ChevronRightIcon,
  ExternalLinkIcon,
  LockClosedIcon,
} from "@radix-ui/react-icons";
import { InsightsSummary } from "./InsightsSummary";

export function SmallInsightsSummary({ onViewAll }: { onViewAll: () => void }) {
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const insights = useInsightsSummary();
  const { from } = useInsightsPeriod();
  const { isLoading, subscription } = useTeamOrbSubscription(team?.id);
  const log = useLogDeploymentEvent();

  if (deployment?.kind === "local") {
    return null;
  }

  if (!subscription && !isLoading) {
    return (
      <HealthCard title="Insights" size="lg">
        <div className="relative h-[18.5rem] w-full">
          <div className="pointer-events-none absolute inset-0 select-none bg-background-secondary blur-[2px]">
            <InsightsSummary insights={UPSELL_INSIGHTS} />
          </div>
          <span className="absolute inset-0 z-10 flex items-center justify-center">
            <Sheet
              className="flex max-w-[26rem] flex-col items-center gap-4 bg-background-secondary/85 p-4 shadow-sm"
              padding={false}
            >
              <h4>Insights</h4>
              <p>Get proactive insight into your function health.</p>
              {insights ? (
                <p className="animate-fadeInFromLoading">
                  There are currently{" "}
                  <span className="font-semibold">{insights.length}</span>{" "}
                  actionable Insight
                  {insights.length !== 1 && "s"} available for this deployment.
                </p>
              ) : (
                <Loading className="h-10 w-96" />
              )}
              <Button
                target="_blank"
                inline
                icon={<ExternalLinkIcon />}
                href="https://docs.convex.dev/dashboard/deployments/health#insights"
                className="text-wrap text-left"
              >
                Learn more about Insights
              </Button>
              <Button
                className="text-xs"
                href={`/t/${team?.slug}/settings/billing`}
                icon={<LockClosedIcon className="size-3" />}
                tip={<>Unlock Insights by upgrading your plan.</>}
                onClick={() =>
                  log("click insights upsell", {
                    numInsights: insights?.length,
                  })
                }
              >
                Upgrade Now
              </Button>
            </Sheet>
          </span>
        </div>
      </HealthCard>
    );
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
