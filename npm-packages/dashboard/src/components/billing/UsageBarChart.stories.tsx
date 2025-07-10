import { Meta, StoryObj } from "@storybook/nextjs";
import { DailyMetric } from "hooks/usageMetrics";
import { Sheet } from "@ui/Sheet";
import { UsageBarChart } from "./UsageBarChart";

const meta: Meta<typeof UsageBarChart> = {
  component: UsageBarChart,
  args: {
    entity: "documents",
  },
  render: (args) => (
    <Sheet>
      <h3 className="mb-4">Chart</h3>
      <UsageBarChart {...args} />
    </Sheet>
  ),
};

export default meta;

const rows: DailyMetric[] = [...Array(31).keys()].map((dayIndex) => ({
  ds: `2023-07-${(dayIndex + 1).toString().padStart(2, "0")}`,
  value: dayIndex === 1 ? 0 : (dayIndex + 5) * 100_000,
}));

export const Standard: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: rows.slice(0, 15),
  },
};

export const FullMonth: StoryObj<typeof UsageBarChart> = {
  args: {
    rows,
  },
};

export const FewDays: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: rows.slice(0, 3),
  },
};

export const HighValuesOnly: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: [
      { ds: "2023-06-22", value: 250 },
      { ds: "2023-06-23", value: 300 },
    ],
  },
};

export const SingleEntry: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: rows.slice(0, 1),
  },
};

export const Empty: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: [],
  },
};

export const MissingEntries: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: [...rows.slice(20, 30), ...rows.slice(0, 10)],
  },
};

export const Storage: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: rows.map((row) => ({ ...row, value: row.value * 100 })),
    quantityType: "storage",
  },
};

export const ActionCompute: StoryObj<typeof UsageBarChart> = {
  args: {
    rows: rows.map((row) => ({ ...row, value: row.value * 100 })),
    quantityType: "actionCompute",
  },
};
