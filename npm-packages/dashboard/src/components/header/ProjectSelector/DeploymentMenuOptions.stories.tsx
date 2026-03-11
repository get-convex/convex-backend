import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
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
        name: "dev-deployment",
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
        name: "dev-deployment",
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
        name: "prod-default",
        deploymentType: "prod",
        isDefault: true,
      }),
      // Default dev — shows "Development (Cloud)"
      createCloudDeployment({
        name: "dev-default",
        deploymentType: "dev",
        isDefault: true,
        creator: 1,
        reference: "dev-default",
      }),
      // Non-default dev — shows reference instead
      createCloudDeployment({
        name: "dev-secondary",
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
        name: "prod-default",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "prod-secondary",
        deploymentType: "prod",
        isDefault: false,
      }),
      createCloudDeployment({
        name: "prod-tertiary",
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
        name: "prod-deployment",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "custom-staging",
        deploymentType: "custom",
      }),
      createCloudDeployment({
        name: "custom-qa",
        deploymentType: "custom",
      }),
    ],
  },
};

export const WithPreviewDeployments: Story = {
  args: {
    deployments: [
      createCloudDeployment({
        name: "prod-deployment",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "preview-feature-login",
        deploymentType: "preview",
        previewIdentifier: "feature/new-login",
      }),
      createCloudDeployment({
        name: "preview-fix-bug",
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
        name: "prod-default",
        deploymentType: "prod",
        isDefault: true,
      }),
      createCloudDeployment({
        name: "prod-secondary",
        deploymentType: "prod",
        isDefault: false,
      }),
      // Dev deployment (default - shows "Development (Cloud)")
      createCloudDeployment({
        name: "dev-deployment",
        deploymentType: "dev",
        isDefault: true,
        creator: 1,
      }),
      // Non-default dev deployment (shows reference, not "Development (Cloud)")
      createCloudDeployment({
        name: "dev-secondary",
        deploymentType: "dev",
        isDefault: false,
        creator: 1,
        reference: "dev-secondary",
      }),
      // Previews
      createCloudDeployment({
        name: "preview-feature",
        deploymentType: "preview",
        previewIdentifier: "feature/awesome",
      }),
      // Custom
      createCloudDeployment({
        name: "custom-staging",
        deploymentType: "custom",
      }),
    ],
  },
};

export const NonDefaultTeammateDev: Story = {
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
        name: "prod-default",
        deploymentType: "prod",
        isDefault: true,
      }),
      // Teammate's default dev — shows "Jane Smith's dev"
      createCloudDeployment({
        name: "teammate-dev-default",
        deploymentType: "dev",
        isDefault: true,
        creator: 2,
        reference: "teammate-dev-default",
      }),
      // Teammate's non-default dev — shows reference only
      createCloudDeployment({
        name: "teammate-dev-secondary",
        deploymentType: "dev",
        isDefault: false,
        creator: 2,
        reference: "teammate-dev-secondary",
      }),
    ],
  },
};
