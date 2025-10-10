import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { ProgressBar } from "@ui/ProgressBar";
import { ProjectDetails, Team } from "generatedApi";
import { formatQuantity, QuantityType } from "./lib/formatQuantity";
import { ProjectLink } from "./ProjectLink";

export interface DailyChartDetailItem {
  project?: ProjectDetails | null;
  name?: string; // For categories (when not a project)
  value: number;
  color: string;
}

export function DailyChartDetailView({
  date,
  items,
  quantityType,
  onBack,
  team,
  memberId,
}: {
  date: number;
  items: DailyChartDetailItem[];
  quantityType: QuantityType;
  onBack: () => void;
  team?: Team;
  memberId?: number;
}) {
  // Calculate total for percentage calculations
  const total = items.reduce((sum, item) => sum + item.value, 0);

  // Sort items by value descending and filter out zero values
  const sortedItems = items
    .filter((item) => item.value > 0)
    .sort((a, b) => b.value - a.value);

  const dateString = new Date(date).toLocaleDateString("en-us", {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  });

  return (
    <div className="flex h-full w-full animate-fadeInFromLoading flex-col bg-background-secondary">
      {/* Header */}
      <div className="flex items-center gap-3 border-b px-4 py-3">
        <Button
          size="sm"
          variant="neutral"
          icon={<ArrowLeftIcon />}
          inline
          tip="Go back"
          onClick={onBack}
          tabIndex={0}
          aria-label="Go back"
        />
        <div>
          <h2 className="text-base font-semibold text-content-primary">
            {dateString}
          </h2>
          <p className="text-xs text-content-secondary">
            {formatQuantity(total, quantityType)} total
          </p>
        </div>
      </div>

      {/* Items List */}
      <div className="scrollbar flex-1 overflow-y-auto p-4">
        <div className="flex flex-col gap-4 overflow-hidden p-0">
          {sortedItems.map((item, index) => {
            const percentage = total > 0 ? (item.value / total) * 100 : 0;
            const displayName =
              item.project !== undefined
                ? (item.project?.name ?? "Deleted Project")
                : (item.name ?? "Unknown");

            return (
              <div key={`${item.project?.id ?? item.name ?? index}`}>
                <div className="mb-2 flex items-center justify-between gap-4">
                  {item.project !== undefined ? (
                    <ProjectLink
                      project={item.project}
                      team={team}
                      memberId={memberId}
                    />
                  ) : (
                    <span className="text-sm font-medium text-content-primary">
                      {item.name}
                    </span>
                  )}
                  <span className="text-sm font-semibold text-content-primary tabular-nums">
                    {formatQuantity(item.value, quantityType)}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <ProgressBar
                    fraction={percentage / 100}
                    ariaLabel={`${displayName} usage percentage`}
                    variant="solid"
                    className="h-2 flex-1"
                  />
                  <span className="min-w-[3ch] text-right text-xs text-content-secondary tabular-nums">
                    {percentage.toFixed(0)}%
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
