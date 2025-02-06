import { InsightsSummaryData } from "api/insights";
import { Sheet } from "dashboard-common/elements/Sheet";
import { Loading } from "dashboard-common/elements/Loading";
import { ProblemForInsight } from "./ProblemForInsight";
import { ChartForInsight } from "./ChartForInsight";
import { EventsForInsight } from "./EventsForInsight";

export function InsightSummaryBreakdown({
  insight,
}: {
  insight?: InsightsSummaryData | null;
}) {
  if (!insight) {
    return (
      <Sheet className="flex h-full w-full items-center justify-center text-content-secondary">
        {insight === null ? (
          "Insight not found."
        ) : (
          <Loading className="w-full" />
        )}
      </Sheet>
    );
  }
  return (
    <Sheet className="flex max-h-full min-h-[40rem] max-w-full flex-col gap-4 overflow-y-auto scrollbar">
      <div className="flex items-end justify-between gap-4">
        <ProblemForInsight insight={insight} explain />
      </div>
      <ChartForInsight insight={insight} />
      <EventsForInsight insight={insight} />
    </Sheet>
  );
}
