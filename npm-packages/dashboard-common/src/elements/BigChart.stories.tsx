import type { Meta, StoryObj } from "@storybook/nextjs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { Sheet } from "@ui/Sheet";
import { ChartData } from "@common/lib/charts/types";
import { BigChart } from "./BigChart";

const makeChartData = (label: string, color: string): ChartData => ({
  xAxisKey: "time",
  data: [
    { time: "12:00 PM", [label]: 120 },
    { time: "12:01 PM", [label]: 250 },
    { time: "12:02 PM", [label]: 90 },
    { time: "12:03 PM", [label]: 180 },
  ],
  lineKeys: [{ key: label, name: ` ${label}`, color }],
});

const meta = {
  component: BigChart,
  args: {
    labels: ["Requests"],
    syncId: "storybook",
    dataSources: [async () => makeChartData("requests", "var(--chart-line-1)")],
  },
  render: (args) => (
    <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
      <Sheet>
        <BigChart {...args} />
      </Sheet>
    </DeploymentInfoContext.Provider>
  ),
} satisfies Meta<typeof BigChart>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Single: Story = {};

export const Multiple: Story = {
  args: {
    labels: ["Requests", "Errors"],
    dataSources: [
      async () => makeChartData("requests", "var(--chart-line-2)"),
      async () => makeChartData("errors", "var(--chart-line-3)"),
    ],
  },
};
