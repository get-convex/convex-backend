import type { Meta, StoryObj } from "@storybook/react";
import { SpendingLimitsForm } from "./SpendingLimits";

const meta: Meta<typeof SpendingLimitsForm> = {
  component: SpendingLimitsForm,
};

export default meta;
type Story = StoryObj<typeof SpendingLimitsForm>;

export const DefaultWithSpendingLimitEnabled: Story = {
  args: {
    defaultValue: {
      spendingLimitEnabled: true,
      spendingLimitDisableThresholdUsd: null,
      spendingLimitWarningThresholdUsd: null,
    },
    currentSpendingUsd: 0,
  },
};

export const DefaultWithSpendingLimitDisabled: Story = {
  args: {
    defaultValue: {
      spendingLimitEnabled: false,
      spendingLimitDisableThresholdUsd: null,
      spendingLimitWarningThresholdUsd: null,
    },
    currentSpendingUsd: 0,
  },
};

export const DefaultWithHighCurrentSpending: Story = {
  args: {
    defaultValue: {
      spendingLimitEnabled: true,
      spendingLimitDisableThresholdUsd: null,
      spendingLimitWarningThresholdUsd: null,
    },
    currentSpendingUsd: 1234,
  },
};

export const Loading: Story = {
  args: {
    defaultValue: undefined,
    currentSpendingUsd: 0,
  },
};

export const ZeroUsageSpending: Story = {
  args: {
    defaultValue: {
      spendingLimitEnabled: true,
      spendingLimitDisableThresholdUsd: 0,
      spendingLimitWarningThresholdUsd: null,
    },
    currentSpendingUsd: 0,
  },
};

export const DisabledWithWarningThreshold: Story = {
  args: {
    defaultValue: {
      spendingLimitEnabled: false,
      spendingLimitDisableThresholdUsd: null,
      spendingLimitWarningThresholdUsd: 20,
    },
    currentSpendingUsd: 25,
  },
};

export const EnabledWithWarningThreshold: Story = {
  args: {
    defaultValue: {
      spendingLimitEnabled: true,
      spendingLimitDisableThresholdUsd: 30,
      spendingLimitWarningThresholdUsd: 20,
    },
    currentSpendingUsd: 25,
  },
};
