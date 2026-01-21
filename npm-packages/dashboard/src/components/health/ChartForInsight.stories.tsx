import type { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { Insight } from "api/insights";
import { Sheet } from "@ui/Sheet";
import { ChartForInsight } from "./ChartForInsight";

const mockClient = mockConvexReactClient().registerQueryFake(
  udfs.paginatedDeploymentEvents.default,
  () => ({
    page: [],
    isDone: true,
    continueCursor: "",
  }),
);

const now = new Date();

// Format date to match the expected "YYYY-MM-DD HH:00:00" format
function formatHour(date: Date): string {
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}-${String(date.getUTCDate()).padStart(2, "0")} ${String(date.getUTCHours()).padStart(2, "0")}:00:00`;
}

const baseOccInsight: Insight = {
  functionId: "myFunction",
  componentPath: null,
  kind: "occRetried",
  details: {
    occCalls: 5,
    occTableName: "users",
    hourlyCounts: [
      {
        hour: formatHour(new Date(now.getTime() - 2 * 60 * 60 * 1000)),
        count: 2,
      },
      {
        hour: formatHour(new Date(now.getTime() - 1 * 60 * 60 * 1000)),
        count: 3,
      },
    ],
    recentEvents: [],
  },
};

const baseMetricsInsight: Insight = {
  functionId: "myFunction",
  componentPath: null,
  kind: "bytesReadLimit",
  details: {
    count: 10,
    hourlyCounts: [
      {
        hour: formatHour(new Date(now.getTime() - 2 * 60 * 60 * 1000)),
        count: 4,
      },
      {
        hour: formatHour(new Date(now.getTime() - 1 * 60 * 60 * 1000)),
        count: 6,
      },
    ],
    recentEvents: [],
  },
};

const meta = {
  component: ChartForInsight,
  args: {
    insight: baseOccInsight,
  },
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <Sheet>
          <ChartForInsight {...args} />
        </Sheet>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} satisfies Meta<typeof ChartForInsight>;

export default meta;
type Story = StoryObj<typeof meta>;

export const OccRetried: Story = {
  args: {
    insight: baseOccInsight,
  },
};

export const OccFailedPermanently: Story = {
  args: {
    insight: {
      ...baseOccInsight,
      kind: "occFailedPermanently",
    },
  },
};

export const BytesReadLimit: Story = {
  args: {
    insight: baseMetricsInsight,
  },
};

export const BytesReadThreshold: Story = {
  args: {
    insight: {
      ...baseMetricsInsight,
      kind: "bytesReadThreshold",
    },
  },
};

export const DocumentsReadLimit: Story = {
  args: {
    insight: {
      ...baseMetricsInsight,
      kind: "documentsReadLimit",
    },
  },
};

export const DocumentsReadThreshold: Story = {
  args: {
    insight: {
      ...baseMetricsInsight,
      kind: "documentsReadThreshold",
    },
  },
};
