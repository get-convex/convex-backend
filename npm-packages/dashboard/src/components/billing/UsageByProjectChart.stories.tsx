import type { Meta, StoryObj } from "@storybook/nextjs";
import { useState } from "react";
import { DailyMetricByProject } from "hooks/usageMetrics";
import { Sheet } from "@ui/Sheet";
import { UsageByProjectChart } from "./UsageByProjectChart";

function UsageByProjectChartWrapper(
  args: Omit<
    React.ComponentProps<typeof UsageByProjectChart>,
    "selectedDate" | "setSelectedDate"
  >,
) {
  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  return (
    <UsageByProjectChart
      {...args}
      selectedDate={selectedDate}
      setSelectedDate={setSelectedDate}
    />
  );
}

const rows: DailyMetricByProject[] = [...Array(14).keys()].flatMap(
  (dayIndex) => {
    const ds = `2023-07-${(dayIndex + 1).toString().padStart(2, "0")}`;
    return [
      { ds, projectId: 1, value: (dayIndex + 1) * 1000 },
      { ds, projectId: 2, value: (14 - dayIndex) * 800 },
      { ds, projectId: "_rest", value: dayIndex * 150 },
    ];
  },
);

const meta = {
  component: UsageByProjectChartWrapper,
  args: {
    rows,
    quantityType: "unit",
    team: {
      id: 42,
      name: "My team",
      creator: 1,
      slug: "my-team",
      suspended: false,
      referralCode: "TEAM123",
      referredBy: null,
    },
  },
  render: (args) => (
    <Sheet>
      <h3 className="mb-4">Chart</h3>
      <UsageByProjectChartWrapper {...args} />
    </Sheet>
  ),
} satisfies Meta<typeof UsageByProjectChartWrapper>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Standard: Story = {};

export const ActionCompute: Story = {
  args: {
    quantityType: "actionCompute",
  },
};

export const SingleDay: Story = {
  args: {
    rows: rows.filter((r) => r.ds === "2023-07-07"),
  },
};
