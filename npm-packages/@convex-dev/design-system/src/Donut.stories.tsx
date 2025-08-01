import type { Meta, StoryObj } from "@storybook/nextjs";

import { Donut } from "./Donut";

const meta = {
  component: Donut,
} satisfies Meta<typeof Donut>;

export default meta;
type Story = StoryObj<typeof meta>;

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
