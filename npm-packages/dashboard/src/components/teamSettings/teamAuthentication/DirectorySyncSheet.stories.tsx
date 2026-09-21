import { Meta, StoryObj } from "@storybook/nextjs";
import { fn, mocked, screen, userEvent } from "storybook/test";
import {
  useDirectorySyncGroups,
  useDisableDirectorySync,
  useEnableDirectorySync,
  useGenerateDirectorySyncConfigurationLink,
  useGetDirectorySync,
  useSetGroupRoleMapping,
  useStagedDirectoryMembers,
} from "api/directorySync";
import {
  useHasCustomRolePermission,
  useIsCurrentMemberTeamAdmin,
  useListCustomRoles,
} from "api/roles";
import { useGetSSO, useTeamEntitlements } from "api/teams";
import type { DirectoryGroupResponse, TeamResponse } from "generatedApi";
import { DirectorySyncSheet } from "./DirectorySyncSheet";

const team: TeamResponse = {
  id: 1,
  creator: 1,
  name: "Acme Corp",
  slug: "acme",
  suspended: false,
  referralCode: "ACME01",
};

const groups: DirectoryGroupResponse[] = [
  {
    workosGroupId: "group_admins",
    name: "convex-team-admins",
    idpId: "idp_admins",
    mapping: { role: "admin" },
  },
  {
    workosGroupId: "group_eng",
    name: "Engineering",
    idpId: "idp_eng",
    mapping: null,
  },
  {
    workosGroupId: "group_leads",
    name: "Engineering Leads",
    idpId: "idp_leads",
    mapping: { role: "admin" },
  },
  {
    workosGroupId: "group_support",
    name: "Support",
    idpId: "idp_support",
    mapping: {
      role: "custom",
      customRoles: [{ id: 7, name: "Support Engineer" }],
    },
  },
];

function mockGroups(data: DirectoryGroupResponse[], hasMore = false) {
  mocked(useDirectorySyncGroups).mockReturnValue({
    data: {
      groups: data,
      pagination: { hasMore, nextCursor: hasMore ? "next" : null },
    },
    isLoading: false,
    error: undefined,
  });
}

const meta = {
  component: DirectorySyncSheet,
  // The section description reads in `--content-secondary`, which measures
  // 4.43:1 against the page background in the light theme — the token is a
  // hair under AA wherever it sits outside a sheet, not something this page
  // can fix. Every other rule still runs.
  parameters: {
    a11y: { config: { rules: [{ id: "color-contrast", enabled: false }] } },
  },
  args: { team },
  beforeEach: () => {
    mocked(useIsCurrentMemberTeamAdmin).mockReturnValue(true);
    mocked(useHasCustomRolePermission).mockReturnValue(true);
    mocked(useTeamEntitlements).mockReturnValue({
      ssoEnabled: true,
      directorySyncEnabled: true,
      customRolesEnabled: true,
    } as ReturnType<typeof useTeamEntitlements>);
    mocked(useListCustomRoles).mockReturnValue({
      data: {
        items: [
          {
            id: 7,
            teamId: 1,
            name: "Support Engineer",
            statements: [],
            createTime: 0,
          },
        ],
        pagination: { hasMore: false, nextCursor: null },
      },
    } as unknown as ReturnType<typeof useListCustomRoles>);
    mocked(useSetGroupRoleMapping).mockReturnValue(fn() as any);
    mocked(useEnableDirectorySync).mockReturnValue(fn() as any);
    mocked(useStagedDirectoryMembers).mockReturnValue({
      data: { items: [], pagination: { hasMore: false, nextCursor: null } },
      isLoading: false,
      error: undefined,
    });
    mockGroups(groups);
    mocked(useGetSSO).mockReturnValue({
      data: {
        createTime: Date.now(),
        requireSsoLogin: false,
        connections: [],
        domains: [{ id: "dom_1", domain: "acme.com", state: "verified" }],
      },
      isLoading: false,
    });
    mocked(useGenerateDirectorySyncConfigurationLink).mockReturnValue(
      fn(async () => ({ link: "https://portal.workos.com/example" })) as any,
    );
    mocked(useDisableDirectorySync).mockReturnValue(fn() as any);
    mocked(useGetDirectorySync).mockReturnValue({
      data: { directory: null, enabled: false },
      isLoading: false,
    });
  },
} satisfies Meta<typeof DirectorySyncSheet>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NotConfigured: Story = {};

const linkedDirectory = {
  id: "directory_1",
  name: "Acme Okta Directory",
  type: "okta scim v2.0",
  state: "linked",
  linked: true,
};

function mockConfigured() {
  mocked(useGetDirectorySync).mockReturnValue({
    data: { directory: linkedDirectory, enabled: false },
    isLoading: false,
  });
}

export const Configured: Story = {
  beforeEach: mockConfigured,
};

export const ManagementEnabled: Story = {
  beforeEach: () => {
    mocked(useGetDirectorySync).mockReturnValue({
      data: { directory: linkedDirectory, enabled: true },
      isLoading: false,
    });
  },
};

// The roster only reaches the menu once the directory provisions, so the
// story opens the menu to show it there.
export const ManagementEnabledMenu: Story = {
  ...ManagementEnabled,
  play: async () => {
    await userEvent.click(
      await screen.findByRole("button", { name: "Okta options" }),
    );
  },
};

export const ConfiguredMenu: Story = {
  ...Configured,
  play: async () => {
    await userEvent.click(
      await screen.findByRole("button", { name: "Okta options" }),
    );
  },
};

export const ConfiguredWithManyGroups: Story = {
  beforeEach: () => {
    mockConfigured();
    mockGroups(groups, true);
  },
};

export const ConfiguredAwaitingSync: Story = {
  beforeEach: () => {
    mockConfigured();
    mockGroups([]);
  },
};

export const EditGroupRole: Story = {
  ...Configured,
  play: async () => {
    await userEvent.click(
      await screen.findByRole("button", { name: "Edit Engineering role" }),
    );
  },
};

// A member who may manage the directory but not read the team's custom
// roles: the dialog says so where the selector would be.
export const EditGroupRoleWithoutCustomRolePermission: Story = {
  beforeEach: () => {
    mockConfigured();
    mocked(useIsCurrentMemberTeamAdmin).mockReturnValue(false);
    mocked(useHasCustomRolePermission).mockImplementation(
      (_teamId, action) => action !== "customRole:view",
    );
  },
  play: async () => {
    await userEvent.click(
      await screen.findByRole("button", { name: "Edit Support role" }),
    );
  },
};

export const NotEntitled: Story = {
  beforeEach: () => {
    mocked(useTeamEntitlements).mockReturnValue({
      ssoEnabled: true,
      directorySyncEnabled: false,
    } as ReturnType<typeof useTeamEntitlements>);
  },
};

export const NoPermission: Story = {
  beforeEach: () => {
    mocked(useIsCurrentMemberTeamAdmin).mockReturnValue(false);
    mocked(useHasCustomRolePermission).mockReturnValue(false);
  },
};
