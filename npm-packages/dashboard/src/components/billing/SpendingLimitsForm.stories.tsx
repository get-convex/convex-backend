import type { Meta, StoryObj } from "@storybook/react";
import { SpendingLimitsForm } from "./SpendingLimits";

const meta: Meta<typeof SpendingLimitsForm> = {
  component: SpendingLimitsForm,
};

export default meta;
type Story = StoryObj<typeof SpendingLimitsForm>;

export const Default: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: undefined,
      spendingLimitDisableThresholdUsd: null,
    },
    currentSpendingUsd: 0,
  },
};

export const BothThresholdsDisabled: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: null,
      spendingLimitDisableThresholdUsd: null,
    },
    currentSpendingUsd: 0,
  },
};

export const BothThresholdsEmpty: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: undefined,
      spendingLimitDisableThresholdUsd: undefined,
    },
    currentSpendingUsd: 0,
  },
};

export const DisableThresholdOnly: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: null,
      spendingLimitDisableThresholdUsd: 100,
    },
    currentSpendingUsd: 0,
  },
};

export const WarningThresholdOnly: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: 100,
      spendingLimitDisableThresholdUsd: null,
    },
    currentSpendingUsd: 0,
  },
};

export const HighCurrentSpending: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: null,
      spendingLimitDisableThresholdUsd: undefined,
    },
    currentSpendingUsd: 1234,
  },
};

export const ZeroUsageSpending: Story = {
  args: {
    defaultValue: {
      spendingLimitWarningThresholdUsd: null,
      spendingLimitDisableThresholdUsd: 0,
    },
    currentSpendingUsd: 0,
  },
};

export const Loading: Story = {
  args: {
    defaultValue: undefined,
    currentSpendingUsd: 0,
  },
};
