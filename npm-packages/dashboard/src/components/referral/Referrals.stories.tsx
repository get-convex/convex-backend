import type { Meta, StoryObj } from "@storybook/react";
import { ReferralState } from "generatedApi";
import { ReferralsInner } from "./Referrals";

const meta: Meta<typeof ReferralsInner> = {
  component: ReferralsInner,
  tags: ["autodocs"],
  args: {
    referralCode: "CONVEX123",
  },
};

export default meta;
type Story = StoryObj<typeof ReferralsInner>;

const mockReferralState: ReferralState = {
  verifiedReferrals: ["Team A", "Team B"],
  pendingReferrals: ["Team C", "Team D"],
  referredBy: null,
};

export const FreePlan: Story = {
  args: {
    isPaidPlan: false,
    referralState: mockReferralState,
  },
};

export const PaidPlan: Story = {
  args: {
    isPaidPlan: true,
    referralState: mockReferralState,
  },
};

export const Loading: Story = {
  args: {
    isPaidPlan: undefined,
    referralState: undefined,
  },
};
