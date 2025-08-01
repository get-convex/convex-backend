import { Meta, StoryObj } from "@storybook/nextjs";
import { DailyPerTagMetrics } from "hooks/usageMetrics";
import { Sheet } from "@ui/Sheet";
import { UsageStackedBarChart } from "./UsageBarChart";

const meta = {
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
} satisfies Meta<typeof UsageStackedBarChart>;

export default meta;
type Story = StoryObj<typeof meta>;

const rows: DailyPerTagMetrics[] = [...Array(31).keys()].map((dayIndex) => ({
  ds: `2023-07-${(dayIndex + 1).toString().padStart(2, "0")}`,
  metrics: ["cats", "dogs", "puppies"].map((tag) => ({
    tag,
    value: Math.floor(Math.random() * 100000),
  })),
  categoryRenames: { puppies: "dogs" },
}));

export const Standard: Story = {
  args: {
    rows: rows.slice(0, 15),
  },
};

export const FullMonth: Story = {
  args: {
    rows,
  },
};

export const FewDays: Story = {
  args: {
    rows: rows.slice(0, 3),
  },
};

export const SingleEntry: Story = {
  args: {
    rows: rows.slice(0, 1),
  },
};

export const Empty: Story = {
  args: {
    rows: [],
  },
};

export const MissingEntries: Story = {
  args: {
    rows: [...rows.slice(20, 30), ...rows.slice(0, 10)],
  },
};

export const Storage: Story = {
  args: {
    rows,
    quantityType: "storage",
    showCategoryTotals: false,
  },
};

export const ActionCompute: Story = {
  args: {
    rows,
    quantityType: "actionCompute",
  },
};
