import { InsightsSummaryData } from "api/insights";
import { Loading } from "dashboard-common";
import { CheckCircledIcon } from "@radix-ui/react-icons";
import { EmptySection } from "elements/EmptySection";
import { InsightsSummaryListItem } from "./InsightsSummaryListItem";

export function InsightsSummary({
  insights,
}: {
  insights: InsightsSummaryData[] | undefined;
}) {
  const hasAnyInsights = !insights || insights.length > 0;

  if (!hasAnyInsights) {
    return (
      <div className="py-16">
        <EmptySection
          Icon={CheckCircledIcon}
          header="All clear!"
          color="green"
          sheet={false}
          body="There are no issues here to address."
          learnMoreButton={{
            href: "https://docs.convex.dev/dashboard/deployments/health#insights",
            children: "Learn more about Insights",
          }}
        />
      </div>
    );
  }
  return (
    <div className="flex h-full w-full animate-fadeInFromLoading flex-col overflow-hidden">
      <div className="flex gap-2 border-b px-2 pb-1 pt-2 text-xs text-content-secondary">
        <p className="w-20">Severity</p>
        <p className="w-72">Function</p>
        <p className="w-60">Problem</p>
        <p className="w-60">Chart</p>
      </div>
      {insights && (
        <div className="flex w-full animate-fadeInFromLoading flex-col overflow-auto scrollbar">
          {insights.map((insight, idx) => (
            <InsightsSummaryListItem key={idx} insight={insight} />
          ))}
        </div>
      )}

      {!insights &&
        Array.from(
          {
            length: 5,
          },
          (_, i) => i,
        ).map((i) => (
          <div
            key={i}
            className="flex w-full items-center gap-2 border-b p-2 last:border-b-0"
          >
            <Loading className="h-6 w-20" />
            <Loading className="h-4 w-72" />
            <Loading className="h-9 w-60" />
            <Loading className="h-9 w-60" />
          </div>
        ))}
    </div>
  );
}
