import { Meta, StoryObj } from "@storybook/nextjs";
import type { CustomRoleResponse } from "@convex-dev/platform/managementApi";
import { TeamMember } from "generatedApi";

import { EditTeamRoleDialog } from "./EditTeamRoleDialog";

const meta = { component: EditTeamRoleDialog } satisfies Meta<
  typeof EditTeamRoleDialog
>;

export default meta;
type Story = StoryObj<typeof meta>;

const developer: TeamMember = {
  id: 1,
  email: "user1@example.org",
  name: "Developer User",
  role: "developer",
  customRoles: [],
};

const admin: TeamMember = {
  id: 2,
  email: "user2@example.org",
  name: "Admin User",
  role: "admin",
  customRoles: [],
};

const customMember: TeamMember = {
  id: 3,
  email: "user3@example.org",
  name: "Custom User",
  role: "custom",
  customRoles: [
    { id: 10, name: "Project Auditor" },
    { id: 20, name: "Billing Reader" },
  ],
};

const customRoles: CustomRoleResponse[] = [
  {
    id: 10,
    teamId: 1,
    name: "Project Auditor",
    description: null,
    statements: [],
    creator: null,
    createTime: 0,
  },
  {
    id: 20,
    teamId: 1,
    name: "Billing Reader",
    description: null,
    statements: [],
    creator: null,
    createTime: 0,
  },
  {
    id: 30,
    teamId: 1,
    name: "Deploy Operator",
    description: null,
    statements: [],
    creator: null,
    createTime: 0,
  },
];

const onSave = async () => {};
const onClose = () => {};

export const DeveloperMember: Story = {
  args: {
    member: developer,
    customRoles,
    customRolesEnabled: true,
    customRolesVisible: true,
    onSave,
    onClose,
  },
};

export const AdminMember: Story = {
  args: {
    member: admin,
    customRoles,
    customRolesEnabled: true,
    customRolesVisible: true,
    onSave,
    onClose,
  },
};

export const CustomMember: Story = {
  args: {
    member: customMember,
    customRoles,
    customRolesEnabled: true,
    customRolesVisible: true,
    onSave,
    onClose,
  },
};

export const AllCustomRolesAssigned: Story = {
  args: {
    member: {
      ...customMember,
      customRoles: [
        { id: 10, name: "Project Auditor" },
        { id: 20, name: "Billing Reader" },
        { id: 30, name: "Deploy Operator" },
      ],
    },
    customRoles,
    customRolesEnabled: true,
    customRolesVisible: true,
    onSave,
    onClose,
  },
};

export const EntitlementOff: Story = {
  args: {
    member: developer,
    customRoles: [],
    customRolesEnabled: false,
    customRolesVisible: true,
    onSave,
    onClose,
  },
};

export const FlagOff: Story = {
  args: {
    member: developer,
    customRoles: [],
    customRolesEnabled: false,
    customRolesVisible: false,
    onSave,
    onClose,
  },
};

export const NoCustomRolesExist: Story = {
  args: {
    member: { ...customMember, customRoles: [] },
    customRoles: [],
    customRolesEnabled: true,
    customRolesVisible: true,
    onSave,
    onClose,
  },
};
