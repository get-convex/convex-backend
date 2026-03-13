import { Meta, StoryObj } from "@storybook/nextjs";
import { fn } from "storybook/test";
import { SegmentedControl } from "./SegmentedControl";

const meta = {
  component: SegmentedControl,
  args: {
    onChange: fn(),
  },
} satisfies Meta<typeof SegmentedControl>;

export default meta;
type Story = StoryObj<typeof meta>;

export const TwoOptions: Story = {
  args: {
    options: [
      { label: "Projects", value: "projects" },
      { label: "Deployments", value: "deployments" },
    ],
    value: "projects",
  },
};

export const TwoOptionsSecondSelected: Story = {
  args: {
    options: [
      { label: "Projects", value: "projects" },
      { label: "Deployments", value: "deployments" },
    ],
    value: "deployments",
  },
};

export const ThreeOptions: Story = {
  args: {
    options: [
      { label: "Day", value: "day" },
      { label: "Week", value: "week" },
      { label: "Month", value: "month" },
    ],
    value: "week",
  },
};

export const FourOptions: Story = {
  args: {
    options: [
      { label: "All", value: "all" },
      { label: "Success", value: "success" },
      { label: "Error", value: "error" },
      { label: "Pending", value: "pending" },
    ],
    value: "all",
  },
};
