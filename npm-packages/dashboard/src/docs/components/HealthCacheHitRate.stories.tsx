import type { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import { mocked } from "storybook/test";
import udfs from "@common/udfs";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import type { ChartData } from "@common/lib/charts/types";
import { useTopKFunctionMetrics } from "@common/lib/appMetrics";
import { CacheHitRate } from "@common/features/health/components/CacheHitRate";

const mockClient = mockConvexReactClient().registerQueryFake(
  udfs.paginatedDeploymentEvents.default,
  () => ({ page: [], isDone: true, continueCursor: "" }),
);

const list = functionIdentifierValue("messages:list");
const channels = functionIdentifierValue("channels:list");
const users = functionIdentifierValue("users:current");

const chartData: ChartData = {
  xAxisKey: "time",
  data: [
    { time: "12:00 PM", [list]: 95, [channels]: 88, [users]: 72 },
    { time: "12:10 PM", [list]: 96, [channels]: 90, [users]: 74 },
    { time: "12:20 PM", [list]: 97, [channels]: 91, [users]: 73 },
    { time: "12:30 PM", [list]: 94, [channels]: 89, [users]: 75 },
    { time: "12:40 PM", [list]: 98, [channels]: 92, [users]: 76 },
    { time: "12:50 PM", [list]: 97, [channels]: 91, [users]: 74 },
  ],
  lineKeys: [
    { key: list, name: list, color: "var(--chart-line-1)" },
    { key: channels, name: channels, color: "var(--chart-line-2)" },
    { key: users, name: users, color: "var(--chart-line-3)" },
  ],
};

const meta = {
  component: CacheHitRate,
  parameters: { a11y: { test: "todo" } },
  beforeEach: () => {
    mocked(useTopKFunctionMetrics).mockReturnValue(chartData);
  },
  render: () => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <div className="max-w-sm">
          <CacheHitRate />
        </div>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} satisfies Meta<typeof CacheHitRate>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
