import type { Meta, StoryObj } from "@storybook/react";

import { UsageBanner } from "./UsageBanner";

const meta: Meta<typeof UsageBanner> = {
  component: UsageBanner,
  args: {
    team: {
      id: 42,
      name: "My team",
      slug: "my-team",
      creator: 1,
      suspended: false,
    },
  },
};

export default meta;
type Story = StoryObj<typeof UsageBanner>;

export const Approaching: Story = {
  args: {
    variant: "Approaching",
  },
};

export const Exceeded: Story = {
  args: {
    variant: "Exceeded",
  },
};

export const Disabled: Story = {
  args: {
    variant: "Disabled",
  },
};

export const Paused: Story = {
  args: {
    variant: "Paused",
  },
};
