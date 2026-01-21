import type { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { Sheet } from "@ui/Sheet";
import { ChartData } from "@common/lib/charts/types";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { ChartForFunctionRate } from "./ChartForFunctionRate";

const mockClient = mockConvexReactClient().registerQueryFake(
  udfs.paginatedDeploymentEvents.default,
  () => ({
    page: [],
    isDone: true,
    continueCursor: "",
  }),
);

const points: Array<{ time: string; queryA: number; queryB: number }> = [
  { time: "12:00 PM", queryA: 12.5, queryB: 80.25 },
  { time: "12:01 PM", queryA: 20.75, queryB: 75 },
  { time: "12:02 PM", queryA: 10, queryB: 60.5 },
];

const functionKeyA = functionIdentifierValue("module.js:queryA");
const functionKeyB = functionIdentifierValue("module.js:queryB");

const chartData: ChartData = {
  xAxisKey: "time",
  data: points.map((row) => ({
    time: row.time,
    [functionKeyA]: row.queryA,
    [functionKeyB]: row.queryB,
  })),
  lineKeys: [
    { key: functionKeyA, name: "queryA", color: "var(--chart-line-1)" },
    { key: functionKeyB, name: "queryB", color: "var(--chart-line-2)" },
  ],
};

const meta = {
  component: ChartForFunctionRate,
  args: {
    chartData,
  },
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <Sheet>
          <div className="h-56">
            <ChartForFunctionRate {...args} />
          </div>
        </Sheet>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} satisfies Meta<typeof ChartForFunctionRate>;

export default meta;
type Story = StoryObj<typeof meta>;

export const CacheHitRate: Story = {
  args: {
    kind: "cacheHitRate",
  },
};

export const FailureRate: Story = {
  args: {
    kind: "failureRate",
  },
};

export const SchedulerStatus: Story = {
  args: {
    kind: "schedulerStatus",
    chartData: {
      ...chartData,
      data: points.map((row) => ({
        time: row.time,
        scheduler: Math.round((row.queryA / 10) * 10),
      })),
      lineKeys: [
        { key: "scheduler", name: "scheduler", color: "var(--chart-line-1)" },
      ],
    },
  },
};

export const Empty: Story = {
  args: {
    chartData: null,
    kind: "cacheHitRate",
  },
};
