import type { Meta, StoryObj } from "@storybook/react";
import { Team } from "generatedApi";
import { RedeemReferralForm } from "./RedeemReferralForm";

const mockTeams: Team[] = [
  {
    id: 1,
    name: "Team 1",
    slug: "team-1",
    creator: 1,
    suspended: false,
    referralCode: "CONVEX123",
  },
  {
    id: 2,
    name: "Team 2",
    slug: "team-2",
    creator: 1,
    suspended: false,
    referralCode: "CONVEX456",
  },
  {
    id: 3,
    name: "Team 3",
    slug: "team-3",
    creator: 1,
    suspended: false,
    referralCode: "CONVEX789",
  },
];

const meta = {
  component: RedeemReferralForm,
  parameters: {
    layout: "centered",
  },
  tags: ["autodocs"],
  args: {
    referralCode: {
      teamName: "Nicolas’s Team",
      valid: true,
      exhausted: false,
    },
    teams: mockTeams,
    selectedTeam: mockTeams[0],
    onTeamSelect: () => {},
    onSubmit: async () => {},
    teamEligibility: { eligible: true },
    onShowTeamSelector: () => {},
    isTeamSelectorShown: false,
    isChef: false,
  },
} satisfies Meta<typeof RedeemReferralForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const LoadingCode: Story = {
  args: {
    referralCode: undefined,
  },
};

export const InvalidCode: Story = {
  args: {
    referralCode: {
      valid: false,
    },
  },
};

export const ExhaustedCode: Story = {
  args: {
    referralCode: {
      teamName: "Nicolas’s Team",
      valid: true,
      exhausted: true,
    },
  },
};

export const LoadingTeams: Story = {
  args: {
    teams: undefined,
  },
};

export const TeamEligibilityLoading: Story = {
  args: {
    teamEligibility: undefined,
  },
};

export const WithPaidSubscription: Story = {
  args: {
    teamEligibility: { eligible: false, reason: "paid_subscription" },
  },
};

export const TeamAlreadyRedeemed: Story = {
  args: {
    teamEligibility: { eligible: false, reason: "already_redeemed" },
  },
};

export const NotAdmin: Story = {
  args: {
    teamEligibility: { eligible: false, reason: "not_admin" },
  },
};

export const ShowingTeamSelector: Story = {
  args: {
    isTeamSelectorShown: true,
  },
};
