import { Meta, StoryObj } from "@storybook/nextjs";
import { DailyPerTagMetrics } from "hooks/usageMetrics";
import { Sheet } from "@ui/Sheet";
import { UsageStackedBarChart } from "./UsageBarChart";

const meta: Meta<typeof UsageStackedBarChart> = {
  component: UsageStackedBarChart,
  args: {
    entity: "animals",
    categories: {
      cats: {
        name: "Cats",
        color: "fill-purple-200 dark:fill-purple-800",
      },
      dogs: {
        name: "Dogs",
        color: "fill-orange-200 dark:fill-orange-800",
      },
    },
  },
  render: (args) => (
    <Sheet>
      <h3 className="mb-4">Chart</h3>
      <UsageStackedBarChart {...args} />
    </Sheet>
  ),
};

export default meta;

const rows: DailyPerTagMetrics[] = [...Array(31).keys()].map((dayIndex) => ({
  ds: `2023-07-${(dayIndex + 1).toString().padStart(2, "0")}`,
  metrics: ["cats", "dogs", "puppies"].map((tag) => ({
    tag,
    value: Math.floor(Math.random() * 100000),
  })),
  categoryRenames: { puppies: "dogs" },
}));

export const Standard: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows: rows.slice(0, 15),
  },
};

export const FullMonth: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows,
  },
};

export const FewDays: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows: rows.slice(0, 3),
  },
};

export const SingleEntry: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows: rows.slice(0, 1),
  },
};

export const Empty: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows: [],
  },
};

export const MissingEntries: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows: [...rows.slice(20, 30), ...rows.slice(0, 10)],
  },
};

export const Storage: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows,
    quantityType: "storage",
    showCategoryTotals: false,
  },
};

export const ActionCompute: StoryObj<typeof UsageStackedBarChart> = {
  args: {
    rows,
    quantityType: "actionCompute",
  },
};
