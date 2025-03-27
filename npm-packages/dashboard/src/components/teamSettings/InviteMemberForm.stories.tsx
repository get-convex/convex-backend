import { StoryObj } from "@storybook/react";

import { TeamMemberResponse } from "generatedApi";
import { InviteMemberForm } from "./InviteMemberForm";

export default { component: InviteMemberForm };

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
export const Primary: StoryObj<typeof InviteMemberForm> = {
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
  },
};
