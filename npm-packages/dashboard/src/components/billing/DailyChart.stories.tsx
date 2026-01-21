import type { Meta, StoryObj } from "@storybook/nextjs";
import { Bar } from "recharts";
import { Sheet } from "@ui/Sheet";
import { DailyChart } from "./DailyChart";

function DailyChartWithBars(
  args: Omit<React.ComponentProps<typeof DailyChart>, "children"> & {
    keys: string[];
  },
) {
  const { keys, ...chartArgs } = args;
  return (
    <DailyChart {...chartArgs}>
      {keys.map((key, idx) => (
        <Bar
          key={key}
          dataKey={key}
          className={idx === 0 ? "fill-chart-line-1" : "fill-chart-line-2"}
          name={key}
          isAnimationActive={false}
          stackId="stack"
        />
      ))}
    </DailyChart>
  );
}

const msInDay = 24 * 60 * 60 * 1000;
const start = Date.UTC(2023, 6, 1);

const baseData = [...Array(14).keys()].map((i) => ({
  dateNumeric: start + i * msInDay,
  cats: i * 10,
  dogs: (14 - i) * 7,
}));

const meta = {
  component: DailyChartWithBars,
  args: {
    keys: ["cats", "dogs"],
    data: baseData,
    quantityType: "unit",
  },
  render: (args) => (
    <Sheet>
      <div className="h-56">
        <DailyChartWithBars {...args} />
      </div>
    </Sheet>
  ),
} satisfies Meta<typeof DailyChartWithBars>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Standard: Story = {};

export const SingleEntry: Story = {
  args: {
    data: [baseData[0]],
  },
};

export const MissingEntries: Story = {
  args: {
    data: baseData.filter((_, idx) => idx % 3 !== 0),
  },
};
