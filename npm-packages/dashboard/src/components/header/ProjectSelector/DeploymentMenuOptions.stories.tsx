import { Meta, StoryObj } from "@storybook/nextjs";
import { screen, expect, mocked, userEvent, waitFor } from "storybook/test";
import { DeploymentResponse, ProjectDetails, TeamResponse } from "generatedApi";
import { ContextMenu } from "@common/features/data/components/ContextMenu";
import { useProfile } from "api/profile";
import { useTeamMembers } from "api/teams";
import { useDefaultDevDeployment } from "api/deployments";
import { DeploymentMenuOptions } from "./DeploymentMenuOptions";

const mockTeam: TeamResponse = {
  id: 1,
  name: "Test Team",
  creator: 1,
  slug: "test-team",
  suspended: false,
  referralCode: "CODE123",
};

const mockProject: ProjectDetails = {
  id: 1,
  name: "Test Project",
  slug: "test-project",
  teamId: 1,
  createTime: Date.now(),
  isDemo: false,
};

let nextId = 0;
function createCloudDeployment(
  overrides: Partial<Extract<DeploymentResponse, { kind: "cloud" }>> & {
    name: string;
    deploymentType: DeploymentResponse["deploymentType"];
  },
): DeploymentResponse {
  return {
    id: nextId++,
    createTime: Date.now(),
    projectId: 1,
    kind: "cloud",
    deploymentClass: "s16",
    region: "us-east-1",
    isDefault: false,
    reference: overrides.name,
    ...overrides,
  } as DeploymentResponse;
}

// Wrapper component to show the menu in an open state
function MenuWrapper({ deployments }: { deployments: DeploymentResponse[] }) {
  return (
    <div style={{ width: "400px", height: "500px" }}>
      <ContextMenu target={{ x: 20, y: 20 }} onClose={() => {}}>
        <DeploymentMenuOptions
          team={mockTeam}
          project={mockProject}
          deployments={deployments}
        />
      </ContextMenu>
    </div>
  );
}

const meta = {
  component: MenuWrapper,
  beforeEach: () => {
    mocked(useProfile).mockReturnValue({
      id: 1,
      name: "Test User",
      email: "test@example.com",
    });
    mocked(useTeamMembers).mockReturnValue([]);
    mocked(useDefaultDevDeployment).mockReturnValue(undefined);
  },
  parameters: { a11y: { test: "todo" } },
} satisfies Meta<typeof MenuWrapper>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NoProdDeployment: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "lazy-panda-202",
        deploymentType: "dev",
        creator: 1,
      }),
    ],
  },
};

export const SingleDefaultProd: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "steady-capybara-123",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "lazy-panda-202",
        deploymentType: "dev",
        creator: 1,
      }),
    ],
  },
};

export const SingleNonDefaultProd: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "steady-capybara-123",
        deploymentType: "prod",
        isDefault: false,
      }),
    ],
  },
};

export const MultipleDevDeployments: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "happy-elephant-101",
        deploymentType: "prod",
        isDefault: true,
      }),
      // Default dev — shows "Development (Cloud)"
      createCloudDeployment({
        name: "gentle-bear-654",
        deploymentType: "dev",
        isDefault: true,
        creator: 1,
        reference: "dev-default",
      }),
      // Non-default dev — shows reference instead
      createCloudDeployment({
        name: "fancy-rabbit-444",
        deploymentType: "dev",
        isDefault: false,
        creator: 1,
        reference: "dev-secondary",
      }),
    ],
  },
};

export const MultipleProds: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "happy-elephant-101",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "calm-dolphin-789",
        deploymentType: "prod",
        isDefault: false,
      }),
      createCloudDeployment({
        name: "quiet-badger-333",
        deploymentType: "prod",
        isDefault: false,
      }),
    ],
  },
};

export const WithCustomDeployments: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "bright-falcon-111",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "swift-tiger-222",
        deploymentType: "custom",
      }),
      createCloudDeployment({
        name: "brave-wolf-100",
        deploymentType: "custom",
      }),
    ],
  },
};

export const WithPreviewDeployments: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "bright-falcon-111",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "quick-lion-987",
        deploymentType: "preview",
        previewIdentifier: "feature/new-login",
      }),
      createCloudDeployment({
        name: "wandering-fish-513",
        deploymentType: "preview",
        previewIdentifier: "fix/bug-123",
      }),
    ],
  },
};

export const FullMenu: Story = {
  args: {
    deployments: [
      // Multiple prods
      createCloudDeployment({
        name: "happy-elephant-101",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "calm-dolphin-789",
        deploymentType: "prod",
        isDefault: false,
      }),
      // Dev deployment (default - shows "Development (Cloud)")
      createCloudDeployment({
        name: "lazy-panda-202",
        deploymentType: "dev",
        isDefault: true,
        creator: 1,
      }),
      // Non-default dev deployment (shows reference, not "Development (Cloud)")
      createCloudDeployment({
        name: "fancy-rabbit-444",
        deploymentType: "dev",
        isDefault: false,
        creator: 1,
        reference: "dev-secondary",
      }),
      // Previews
      createCloudDeployment({
        name: "knowing-antelope-914",
        deploymentType: "preview",
        previewIdentifier: "feature/awesome",
      }),
      // Custom
      createCloudDeployment({
        name: "swift-tiger-222",
        deploymentType: "custom",
      }),
    ],
  },
};

export const NonDefaultDev: Story = {
  play: async () => {
    // Non-default deployments are a main-menu item below 10 items.
    // Use waitFor because the ContextMenu has a fade-in animation (opacity 0→1).
    await waitFor(async () => {
      await expect(screen.getByText("teammate-dev-secondary")).toBeVisible();
    });
    await waitFor(async () => {
      await expect(screen.getByText("dev/vercel")).toBeVisible();
    });
  },
  beforeEach: () => {
    mocked(useTeamMembers).mockReturnValue([
      {
        id: 2,
        name: "Jane Smith",
        email: "jane@example.com",
        role: "developer",
      },
    ]);
  },
  args: {
    deployments: [
      createCloudDeployment({
        name: "calm-panda-321",
        deploymentType: "prod",
        isDefault: true,
      }),
      // Teammate's default dev — shows "Jane Smith's dev"
      createCloudDeployment({
        name: "happy-koala-456",
        deploymentType: "dev",
        isDefault: true,
        creator: 2,
        reference: "teammate-dev-default",
      }),
      // Teammate's non-default dev — shows reference only
      createCloudDeployment({
        name: "swift-eagle-789",
        deploymentType: "dev",
        isDefault: false,
        creator: 2,
        reference: "teammate-dev-secondary",
      }),
      // System non-default dev — shows reference only
      createCloudDeployment({
        name: "quiet-husky-173",
        deploymentType: "dev",
        isDefault: false,
        creator: null,
        reference: "dev/vercel",
      }),
    ],
  },
};

export const NonDefaultDevAndDefaultDev: Story = {
  play: async () => {
    // Non-default deployments are a main-menu item below 10 items.
    // Use waitFor because the ContextMenu has a fade-in animation (opacity 0→1).
    await waitFor(async () => {
      await expect(screen.getByText("teammate-dev-secondary")).toBeVisible();
    });
    await waitFor(async () => {
      await expect(screen.getByText("dev/vercel")).toBeVisible();
    });
  },
  beforeEach: () => {
    mocked(useTeamMembers).mockReturnValue([
      {
        id: 2,
        name: "Jane Smith",
        email: "jane@example.com",
        role: "developer",
      },
    ]);
  },
  args: {
    deployments: [
      createCloudDeployment({
        name: "happy-elephant-101",
        deploymentType: "dev",
        isDefault: true,
        creator: 1,
        reference: "dev/nicolas",
      }),
      // Teammate's default dev — shows "Jane Smith's dev"
      createCloudDeployment({
        name: "happy-koala-456",
        deploymentType: "dev",
        isDefault: true,
        creator: 2,
        reference: "teammate-dev-default",
      }),
      // Teammate's non-default dev — shows reference only
      createCloudDeployment({
        name: "swift-eagle-789",
        deploymentType: "dev",
        isDefault: false,
        creator: 2,
        reference: "teammate-dev-secondary",
      }),
      // System non-default dev — shows reference only
      createCloudDeployment({
        name: "quiet-husky-173",
        deploymentType: "dev",
        isDefault: false,
        creator: null,
        reference: "dev/vercel",
      }),
    ],
  },
};

export const TenNonDefaultTeammateDevsWithoutDefaultDev: Story = {
  play: async () => {
    await expect(
      screen.queryByText("non-default-dev-1"),
    ).not.toBeInTheDocument();
    // Open the "Other Deployments" submenu
    await userEvent.hover(screen.getByText("Other Deployments"));
    // Submenu items appear in a Floating UI portal — query from document.body
    await expect(screen.findByText("non-default-dev-1")).resolves.toBeVisible();
  },
  args: {
    deployments: [
      createCloudDeployment({
        name: "happy-elephant-101",
        deploymentType: "prod",
        isDefault: true,
      }),
      ...Array.from({ length: 10 }, (_, i) =>
        createCloudDeployment({
          name: `brave-husky-1${i + 1}`,
          deploymentType: "dev",
          isDefault: false,
          creator: 2,
          reference: `non-default-dev-${i + 1}`,
        }),
      ),
    ],
  },
};

export const TenNonDefaultTeammateDevsWithDefaultDev: Story = {
  play: async () => {
    await expect(
      screen.queryByText("non-default-dev-1"),
    ).not.toBeInTheDocument();
    // Open the "Other Deployments" submenu
    await userEvent.hover(screen.getByText("Other Deployments"));
    // Submenu items appear in a Floating UI portal — query from document.body
    await expect(screen.findByText("non-default-dev-1")).resolves.toBeVisible();
  },
  args: {
    deployments: [
      createCloudDeployment({
        name: "happy-elephant-101",
        deploymentType: "dev",
        isDefault: true,
        creator: 1,
        reference: "dev/nicolas",
      }),
      ...Array.from({ length: 10 }, (_, i) =>
        createCloudDeployment({
          name: `brave-husky-1${i + 1}`,
          deploymentType: "dev",
          isDefault: false,
          creator: 2,
          reference: `non-default-dev-${i + 1}`,
        }),
      ),
    ],
  },
};
