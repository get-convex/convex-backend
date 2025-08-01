import { Meta, StoryObj } from "@storybook/nextjs";

import { TeamMemberResponse } from "generatedApi";
import { InviteMemberForm } from "./InviteMemberForm";

const meta = { component: InviteMemberForm } satisfies Meta<
  typeof InviteMemberForm
>;

export default meta;
type Story = StoryObj<typeof meta>;

const members: TeamMemberResponse[] = [
  {
    id: 1,
    email: "user1@example.org",
    name: "User 1",
    role: "admin",
  },
  {
    id: 2,
    email: "user2@example.org",
    role: "developer",
  },
];
export const Primary: Story = {
  args: {
    team: {
      creator: 1,
      id: 1,
      name: "team",
      slug: "team",
      suspended: false,
      referralCode: "TEAM123",

      referredBy: null,
    },
    members,
    hasAdminPermissions: true,
  },
};
