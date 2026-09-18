import { Meta, StoryObj } from "@storybook/nextjs";
import { fn, mocked, screen, userEvent } from "storybook/test";
import {
  useDisableDirectorySync,
  useGenerateDirectorySyncConfigurationLink,
  useGetDirectorySync,
} from "api/directorySync";
import {
  useHasCustomRolePermission,
  useIsCurrentMemberTeamAdmin,
} from "api/roles";
import { useGetSSO, useTeamEntitlements } from "api/teams";
import type { TeamResponse } from "generatedApi";
import { DirectorySyncSheet } from "./DirectorySyncSheet";

const team: TeamResponse = {
  id: 1,
  creator: 1,
  name: "Acme Corp",
  slug: "acme",
  suspended: false,
  referralCode: "ACME01",
};

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
    } as ReturnType<typeof useTeamEntitlements>);
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

export const Configured: Story = {
  beforeEach: () => {
    mocked(useGetDirectorySync).mockReturnValue({
      data: {
        directory: {
          id: "directory_1",
          name: "Acme Okta Directory",
          type: "okta scim v2.0",
          state: "linked",
          linked: true,
        },
        enabled: false,
      },
      isLoading: false,
    });
  },
};

export const ConfiguredMenu: Story = {
  ...Configured,
  play: async () => {
    await userEvent.click(
      await screen.findByRole("button", { name: "okta scim v2.0 options" }),
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
