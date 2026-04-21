import { CheckIcon, MixerVerticalIcon } from "@radix-ui/react-icons";
import { HealthCard } from "@common/elements/HealthCard";
import { useTopKFunctionMetrics } from "@common/lib/appMetrics";
import { FunctionRateHeatmapView } from "@common/features/health/components/CategoricalHeatmap";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";
import { Menu, MenuItem } from "@ui/Menu";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";

type ViewMode = "heatmap" | "lineChart";

const VIEW_OPTIONS: { label: string; value: ViewMode }[] = [
  { label: "Heatmap", value: "heatmap" },
  { label: "Line chart", value: "lineChart" },
];

export function CacheHitRate({
  showHeatmaps = false,
}: {
  showHeatmaps?: boolean;
}) {
  const [storedView, setStoredView] = useGlobalLocalStorage<ViewMode>(
    "health-cache-hit-rate-view",
    "heatmap",
  );
  const view = showHeatmaps ? storedView : "lineChart";

  return (
    <HealthCard
      title="Cache Hit Rate"
      tip="The cache hit rate of all query functions."
      action={
        showHeatmaps ? (
          <Menu
            placement="bottom-end"
            buttonProps={{
              "aria-label": "Cache hit rate display options",
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
        <FunctionRateHeatmapView metricKind="cacheHitPercentage" />
      ) : (
        <LineChartView />
      )}
    </HealthCard>
  );
}

function LineChartView() {
  const chartData = useTopKFunctionMetrics("cacheHitPercentage", 3, 60);
  return <ChartForFunctionRate chartData={chartData} kind="cacheHitRate" />;
}
