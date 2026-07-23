import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
import { useEffect } from "react";
import { flagDefaults, useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useCurrentTeam, useTeams } from "api/teams";
import {
  useCurrentProject,
  useInfiniteProjects,
  useProjectById,
} from "api/projects";
import {
  useCurrentDeployment,
  useDeployments,
  usePaginatedDeployments,
} from "api/deployments";
import { useProfile } from "api/profile";
import type { PlatformDeploymentResponse } from "generatedApi";
import { CommandPalette, useCommandPaletteOpen } from "./CommandPalette";

const mockTeam = {
  id: 2,
  creator: 1,
  slug: "acme",
  name: "Acme Corp",
  suspended: false,
  referralCode: "ACME01",
  referredBy: null,
};

const mockProject = {
  id: 7,
  teamId: mockTeam.id,
  name: "My amazing app",
  slug: "my-amazing-app",
  createTime: Date.now(),
  prodDeploymentName: "musical-otter-456",
  devDeploymentName: "happy-capybara-123",
} as NonNullable<ReturnType<typeof useCurrentProject>>;

const otherProjects = [
  { ...mockProject },
  {
    id: 8,
    teamId: mockTeam.id,
    name: "Marketing site",
    slug: "marketing-site",
    createTime: Date.now(),
  },
  {
    id: 9,
    teamId: mockTeam.id,
    name: "Internal tools",
    slug: "internal-tools",
    createTime: Date.now(),
  },
] as NonNullable<ReturnType<typeof useCurrentProject>>[];

const devDeployment: PlatformDeploymentResponse = {
  id: 11,
  name: "happy-capybara-123",
  deploymentType: "dev",
  kind: "cloud",
  isDefault: true,
  projectId: mockProject.id,
  creator: 1,
  createTime: 0,
  class: "s256",
  deploymentUrl: "https://happy-capybara-123.convex.cloud",
  reference: "dev/nicolas",
  region: "aws-us-east-1",
};

const mockProfile = {
  id: 1,
  name: "Nicolas Ettlin",
  email: "nicolas@acme.dev",
};

// The palette's open state lives in a global (so the header trigger can open
// it from anywhere); flip it on when the story mounts so the dialog renders.
function OpenCommandPalette() {
  const [, setOpen] = useCommandPaletteOpen();
  useEffect(() => {
    setOpen(true);
    return () => setOpen(false);
  }, [setOpen]);
  return <CommandPalette />;
}

const meta = {
  component: CommandPalette,
  parameters: {
    layout: "fullscreen",
    // The palette is a focus-trapping Radix dialog rendered over an empty
    // canvas, which trips the automated a11y checks meant for full pages.
    a11y: { test: "todo" },
  },
  render: () => <OpenCommandPalette />,
  beforeEach: () => {
    mocked(useLaunchDarkly).mockReturnValue({
      ...flagDefaults,
      commandPalette: true,
      usageLimits: true,
    });
    mocked(useTeams).mockReturnValue({
      selectedTeamSlug: mockTeam.slug,
      teams: [mockTeam],
    });
    mocked(useCurrentTeam).mockReturnValue(mockTeam);
    mocked(useProfile).mockReturnValue(mockProfile);
    // These hooks are server-backed: their remote rows bypass the palette's
    // client-side filter, so the results must already reflect the query.
    // Filter the mock data by the search argument to match that behavior —
    // otherwise every deployment/project matches every query.
    mocked(useInfiniteProjects).mockImplementation(
      (_teamId, searchQuery = "") => {
        const q = searchQuery.trim().toLowerCase();
        const projects = otherProjects.filter(
          (p) => !q || `${p.name} ${p.slug}`.toLowerCase().includes(q),
        );
        return {
          projects,
          isLoading: false,
          hasMore: false,
          loadMore: () => {},
          debouncedQuery: searchQuery,
          pageSize: 20,
        };
      },
    );
    mocked(usePaginatedDeployments).mockImplementation((_teamId, options) => {
      const q = (options?.q ?? "").trim().toLowerCase();
      const items = [devDeployment].filter(
        (d) =>
          !q ||
          `${"reference" in d ? d.reference : ""} ${d.name}`
            .toLowerCase()
            .includes(q),
      );
      return { items, isLoading: false } as ReturnType<
        typeof usePaginatedDeployments
      >;
    });
    mocked(useDeployments).mockReturnValue({
      deployments: [devDeployment],
      isLoading: false,
    });
    mocked(useProjectById).mockReturnValue({
      project: mockProject,
      isLoading: false,
      error: undefined,
    });
  },
} satisfies Meta<typeof CommandPalette>;

export default meta;
type Story = StoryObj<typeof meta>;

// The root page while viewing a deployment: current page, the deployment's
// pages, the project, the team, and the account/help groups.
export const InsideDeployment: Story = {
  parameters: {
    nextjs: {
      router: {
        pathname: "/t/[team]/[project]/[deploymentName]/data",
        route: "/t/[team]/[project]/[deploymentName]/data",
        asPath: "/t/acme/my-amazing-app/happy-capybara-123/data",
        query: {
          team: "acme",
          project: "my-amazing-app",
          deploymentName: "happy-capybara-123",
        },
      },
    },
  },
  beforeEach: () => {
    mocked(useCurrentProject).mockReturnValue(mockProject);
    mocked(useCurrentDeployment).mockReturnValue(devDeployment);
  },
};

// The root page from a team-level page, with no current project or deployment:
// only the project search, team, and account/help groups.
export const TeamLevel: Story = {
  parameters: {
    nextjs: {
      router: {
        pathname: "/t/[team]",
        route: "/t/[team]",
        asPath: "/t/acme",
        query: { team: "acme" },
      },
    },
  },
  beforeEach: () => {
    mocked(useCurrentProject).mockReturnValue(undefined);
    mocked(useCurrentDeployment).mockReturnValue(undefined);
  },
};
