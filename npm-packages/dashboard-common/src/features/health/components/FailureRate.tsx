import { CheckIcon, MixerVerticalIcon } from "@radix-ui/react-icons";
import { HealthCard } from "@common/elements/HealthCard";
import { useTopKFunctionMetrics } from "@common/lib/appMetrics";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";
import { FunctionRateHeatmapView } from "@common/features/health/components/CategoricalHeatmap";
import type { ChartData } from "@common/lib/charts/types";
import { Menu, MenuItem } from "@ui/Menu";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";

type ViewMode = "heatmap" | "lineChart";

const VIEW_OPTIONS: { label: string; value: ViewMode }[] = [
  { label: "Heatmap", value: "heatmap" },
  { label: "Line chart", value: "lineChart" },
];

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

export function FailureRate({
  showHeatmaps = false,
}: {
  showHeatmaps?: boolean;
}) {
  const [storedView, setStoredView] = useGlobalLocalStorage<ViewMode>(
    "health-failure-rate-view",
    "lineChart",
  );
  const view = showHeatmaps ? storedView : "lineChart";

  return (
    <HealthCard
      title="Failure Rate"
      tip={
        view === "heatmap"
          ? "Failure rate of the worst-performing functions over the last hour."
          : "The failure rate of all your running functions, bucketed by minute."
      }
      action={
        showHeatmaps ? (
          <Menu
            placement="bottom-end"
            buttonProps={{
              "aria-label": "Failure rate display options",
              tip: "Display options",
              size: "xs",
              variant: "neutral",
              inline: true,
              icon: <MixerVerticalIcon className="text-content-tertiary" />,
            }}
          >
            {VIEW_OPTIONS.map((opt) => (
              <MenuItem
                key={`view-${opt.value}`}
                action={() => setStoredView(opt.value)}
              >
                <CheckIcon
                  className={
                    view === opt.value
                      ? "text-content-primary"
                      : "text-transparent"
                  }
                />
                {opt.label}
              </MenuItem>
            ))}
          </Menu>
        ) : undefined
      }
    >
      {view === "heatmap" ? (
        <FunctionRateHeatmapView metricKind="failurePercentage" />
      ) : (
        <LineChartView />
      )}
    </HealthCard>
  );
}

function LineChartView() {
  const chartData = useTopKFunctionMetrics("failurePercentage");
  return <ChartForFunctionRate chartData={chartData} kind="failureRate" />;
}
