import { Meta, StoryObj } from "@storybook/react";
import { Sheet } from "dashboard-common/elements/Sheet";
import { SpendingLimitsSection } from "./SubscriptionOverview";

const meta: Meta<typeof SpendingLimitsSection> = {
  component: SpendingLimitsSection,
  args: {
    hasAdminPermissions: true,
    onSubmit: async () => {},
  },
  render: (args) => (
    <Sheet>
      <SpendingLimitsSection {...args} />
    </Sheet>
  ),
};

export default meta;
type Story = StoryObj<typeof SpendingLimitsSection>;

export const NoSpendingLimits: Story = {
  args: {
    currentSpendLimit: {
      disableThresholdCents: null,
      warningThresholdCents: null,
    },
    currentSpend: { totalCents: 5000, isLoading: false },
  },
};

export const Loading: Story = {
  args: {
    currentSpendLimit: undefined,
    currentSpend: { totalCents: undefined, isLoading: true },
  },
};

export const SomeSpendingLimits: Story = {
  args: {
    currentSpendLimit: {
      disableThresholdCents: 10000,
      warningThresholdCents: 8000,
    },
    currentSpend: { totalCents: 5000, isLoading: false },
  },
};

export const ZeroSpendingLimits: Story = {
  args: {
    currentSpendLimit: {
      disableThresholdCents: 0,
      warningThresholdCents: null,
    },
    currentSpend: { totalCents: 0, isLoading: false },
  },
};

export const WarningOnly: Story = {
  args: {
    currentSpendLimit: {
      disableThresholdCents: null,
      warningThresholdCents: 100_00,
    },
    currentSpend: { totalCents: 0, isLoading: false },
  },
};

export const NoAdminPermissions: Story = {
  args: {
    currentSpendLimit: {
      disableThresholdCents: 50_000_00,
      warningThresholdCents: 42_000_00,
    },
    currentSpend: { totalCents: 5000, isLoading: false },
    hasAdminPermissions: false,
  },
};
