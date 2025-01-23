import type { Meta, StoryObj } from "@storybook/react";

import { Donut } from "./PlanSummary";

const meta: Meta<typeof Donut> = {
  component: Donut,
};

export default meta;
type Story = StoryObj<typeof Donut>;

export const UnderLimit: Story = {
  args: {
    current: 70,
    max: 100,
  },
};

export const OverLimit: Story = {
  args: {
    current: 120,
    max: 100,
  },
};
