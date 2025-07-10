import type { Meta, StoryObj } from "@storybook/nextjs";
import { ReferralsBanner } from "./ReferralsBanner";

const meta = {
  component: ReferralsBanner,
  args: {
    onHide() {},
  },
} satisfies Meta<typeof ReferralsBanner>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    team: {
      id: 1,
      name: "Team 1",
      slug: "team-1",
      creator: 1,
      suspended: false,
      referralCode: "CONVEX123",
    },
    referralState: {
      referrals: ["123", "456", "789"],
      referredBy: "123",
    },
  },
};

export const NoReferrals: Story = {
  args: {
    team: {
      id: 1,
      name: "Team 1",
      slug: "team-1",
      creator: 1,
      suspended: false,
      referralCode: "CONVEX123",
    },
    referralState: {
      referrals: [],
      referredBy: "123",
    },
  },
};

export const FullReferrals: Story = {
  args: {
    team: {
      id: 1,
      name: "Team 1",
      slug: "team-1",
      creator: 1,
      suspended: false,
      referralCode: "CONVEX123",
    },
    referralState: {
      referrals: ["123", "456", "789", "101", "102"],
      referredBy: "123",
    },
  },
};
