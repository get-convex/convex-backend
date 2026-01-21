import type { Meta, StoryObj } from "@storybook/nextjs";

import { SingleGraph } from "./SingleGraph";

const meta = {
  component: SingleGraph,
} satisfies Meta<typeof SingleGraph>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Unit: Story = {
  args: {
    title: "Function Calls",
    data: {
      xAxisKey: "time",
      data: [
        { time: "12:00 PM", value: 120 },
        { time: "12:01 PM", value: 250 },
        { time: "12:02 PM", value: 90 },
        { time: "12:03 PM", value: 180 },
      ],
      lineKeys: [
        { key: "value", name: " function calls", color: "var(--chart-line-1)" },
      ],
    },
  },
};

export const Percentage: Story = {
  args: {
    title: "Cache Hit Rate",
    data: {
      xAxisKey: "time",
      data: [
        { time: "12:00 PM", value: 80 },
        { time: "12:01 PM", value: 90 },
        { time: "12:02 PM", value: 70 },
        { time: "12:03 PM", value: 95 },
      ],
      lineKeys: [{ key: "value", name: "%", color: "var(--chart-line-1)" }],
    },
  },
};

export const Percentile: Story = {
  args: {
    title: "Execution Time",
    syncId: "fnMetrics",
    data: {
      xAxisKey: "time",
      data: [
        { time: "12:00 PM", p50: 42, p90: 67, p95: 89 },
        { time: "12:15 PM", p50: 43, p90: 68, p95: 90 },
        { time: "12:30 PM", p50: 44, p90: 69, p95: 91 },
        { time: "12:45 PM", p50: 44, p90: 70, p95: 92 },
        { time: "1:00 PM", p50: 45, p90: 71, p95: 93 },
        { time: "1:15 PM", p50: 45, p90: 72, p95: 94 },
        { time: "1:30 PM", p50: 44, p90: 71, p95: 93 },
        { time: "1:45 PM", p50: 45, p90: 72, p95: 94 },
        { time: "2:00 PM", p50: 45, p90: 72, p95: 94 },
        { time: "2:15 PM", p50: 44, p90: 71, p95: 93 },
        { time: "2:30 PM", p50: 43, p90: 70, p95: 92 },
        { time: "2:45 PM", p50: 42, p90: 69, p95: 91 },
        { time: "3:00 PM", p50: 41, p90: 68, p95: 90 },
        { time: "3:15 PM", p50: 42, p90: 67, p95: 89 },
        { time: "3:30 PM", p50: 41, p90: 66, p95: 88 },
        { time: "3:45 PM", p50: 42, p90: 67, p95: 89 },
        { time: "4:00 PM", p50: 41, p90: 65, p95: 88 },
      ],
      lineKeys: [
        {
          key: "p50",
          name: "ms p50",
          color: "var(--chart-line-1)",
        },
        {
          key: "p90",
          name: "ms p90",
          color: "var(--chart-line-2)",
        },
        {
          key: "p95",
          name: "ms p95",
          color: "var(--chart-line-3)",
        },
      ],
    },
  },
};
