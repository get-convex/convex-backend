import { Meta, StoryObj } from "@storybook/nextjs";
import { fn, mocked } from "storybook/test";
import {
  useEnableDirectorySync,
  useStagedDirectoryMembers,
} from "api/directorySync";
import type { StagedDirectoryMemberResponse, TeamResponse } from "generatedApi";
import { ReviewDirectoryChangesModal } from "./ReviewDirectoryChangesModal";

const team: TeamResponse = {
  id: 1,
  creator: 1,
  name: "Acme Corp",
  slug: "acme",
  suspended: false,
  referralCode: "ACME01",
};

const items: StagedDirectoryMemberResponse[] = [
  {
    member: { id: 10, name: "Ari", email: "ari@acme.com", role: "admin" },
    directoryUser: {
      directoryUserId: "dir_ari",
      email: "ari@acme.com",
      state: "active",
      groups: [{ workosGroupId: "group_eng", name: "Engineering" }],
      role: "developer",
    },
  },
  {
    member: { id: 11, name: "Sam", email: "sam@acme.com", role: "developer" },
    directoryUser: {
      directoryUserId: "dir_sam",
      email: "sam@acme.com",
      state: "active",
      groups: [{ workosGroupId: "group_support", name: "Support" }],
      role: "custom",
      customRoles: [{ id: 7, name: "Support Engineer" }],
    },
  },
  {
    member: {
      id: 12,
      name: "Contractor",
      email: "contractor@other.com",
      role: "developer",
    },
    directoryUser: null,
  },
  {
    member: {
      id: 13,
      name: "Former Employee",
      email: "former@acme.com",
      role: "developer",
    },
    directoryUser: {
      directoryUserId: "dir_former",
      email: "former@acme.com",
      state: "suspended",
      groups: [],
      role: "developer",
    },
  },
  {
    member: null,
    directoryUser: {
      directoryUserId: "dir_new",
      email: "newcomer@acme.com",
      state: "active",
      groups: [{ workosGroupId: "group_eng", name: "Engineering" }],
      role: "developer",
    },
  },
];

function mockStaged(data: StagedDirectoryMemberResponse[], hasMore = false) {
  mocked(useStagedDirectoryMembers).mockReturnValue({
    data: {
      items: data,
      pagination: { hasMore, nextCursor: hasMore ? "next" : null },
    },
    isLoading: false,
    error: undefined,
  });
}

const meta = {
  component: ReviewDirectoryChangesModal,
  args: { team, enabled: false, onClose: fn() },
  beforeEach: () => {
    mocked(useEnableDirectorySync).mockReturnValue(fn() as any);
    mockStaged(items);
  },
} satisfies Meta<typeof ReviewDirectoryChangesModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Review: Story = {};

export const ReviewManyPages: Story = {
  beforeEach: () => {
    mockStaged(items, true);
  },
};

export const NothingToReview: Story = {
  beforeEach: () => {
    mockStaged([]);
  },
};

export const PendingMembers: Story = {
  args: { enabled: true },
  beforeEach: () => {
    mockStaged(items.filter((item) => item.member === null));
  },
};

export const NoPendingMembers: Story = {
  args: { enabled: true },
  beforeEach: () => {
    mockStaged([]);
  },
};
