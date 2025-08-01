import type { Meta, StoryObj } from "@storybook/nextjs";

import { useState } from "react";
import { Period, UsagePeriodSelector } from "./UsagePeriodSelector";

const currentBillingPeriod = { start: "2023-10-01", end: "2023-10-31" };

const meta = {
  component: UsagePeriodSelector,
  args: {
    currentBillingPeriod,
    onChange: () => {},
  },
} satisfies Meta<typeof UsagePeriodSelector>;

export default meta;
type Story = StoryObj<typeof meta>;

export const CurrentBillingPeriod: Story = {
  args: {
    period: {
      type: "currentBillingPeriod",
      from: "2023-10-01",
      to: "2023-10-31",
    },
  },
};

const currentYear = new Date().getUTCFullYear();
export const PresetPeriod: Story = {
  args: {
    period: {
      type: "presetPeriod",
      from: `${currentYear}-01-01`,
      to: `${currentYear}-12-31`,
    },
  },
};

export const CustomPeriod: Story = {
  args: {
    period: { type: "customPeriod", from: "2023-05-16", to: "2023-08-18" },
  },
};

function InteractiveDemo() {
  const [period, setPeriod] = useState<Period>({
    type: "currentBillingPeriod",
    from: "2023-10-01",
    to: "2023-10-31",
  });
  return (
    <div>
      <pre className="pb-4">{JSON.stringify(period, null, 2)}</pre>
      <UsagePeriodSelector
        period={period}
        onChange={setPeriod}
        currentBillingPeriod={currentBillingPeriod}
      />
    </div>
  );
}

export const Interactive: Story = {
  args: {
    period: {
      type: "currentBillingPeriod",
      from: "2023-10-01",
      to: "2023-10-31",
    },
  },
  render: () => <InteractiveDemo />,
};
