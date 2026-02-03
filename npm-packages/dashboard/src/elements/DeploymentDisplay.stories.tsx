import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
import { DeploymentResponse, ProjectDetails, TeamResponse } from "generatedApi";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { DeploymentLabel } from "./DeploymentDisplay";

// Mock data
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

function createCloudDeployment(
  overrides: Partial<Extract<DeploymentResponse, { kind: "cloud" }>> & {
    name: string;
    deploymentType: DeploymentResponse["deploymentType"];
  },
): DeploymentResponse {
  return {
    id: Math.floor(Math.random() * 1000),
    createTime: Date.now(),
    projectId: 1,
    kind: "cloud",
    region: "us-east-1",
    isDefault: true,
    ...overrides,
  } as DeploymentResponse;
}

function createLocalDeployment(
  overrides: Partial<Extract<DeploymentResponse, { kind: "local" }>> & {
    name: string;
  },
): DeploymentResponse {
  return {
    id: Math.floor(Math.random() * 1000),
    createTime: Date.now(),
    projectId: 1,
    kind: "local",
    deploymentType: "dev",
    deviceName: "MacBook Pro",
    port: 3210,
    isActive: true,
    lastUpdateTime: Date.now(),
    creator: 1,
    isDefault: false,
    ...overrides,
  } as DeploymentResponse;
}

const prodDeployment = createCloudDeployment({
  name: "steady-capybara-123",
  deploymentType: "prod",
  isDefault: true,
});

const devDeployment = createCloudDeployment({
  name: "dev-happy-koala-456",
  deploymentType: "dev",
  creator: 1,
  isDefault: false,
});

const localDevDeployment = createLocalDeployment({
  name: "local-dev-789",
  deviceName: "MacBook Pro",
  port: 3210,
});

const previewDeployment = createCloudDeployment({
  name: "preview-feature-branch",
  deploymentType: "preview",
  previewIdentifier: "feature/new-login",
  isDefault: false,
});

const customDeployment = createCloudDeployment({
  name: "custom-staging-env",
  deploymentType: "custom",
  isDefault: false,
});

const allDeployments = [
  prodDeployment,
  devDeployment,
  localDevDeployment,
  previewDeployment,
  customDeployment,
];

const meta = {
  component: DeploymentLabel,
  decorators: [
    (Story) => (
      <div style={{ width: "500px" }}>
        <Story />
      </div>
    ),
  ],
  render: (args) => <DeploymentLabel {...args} />,
  beforeEach: () => {
    mocked(useCurrentTeam).mockReturnValue(mockTeam);
    mocked(useCurrentProject).mockReturnValue(mockProject);
  },
} satisfies Meta<typeof DeploymentLabel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Production: Story = {
  args: {
    deployment: prodDeployment,
    deployments: allDeployments,
    whoseName: null,
  },
};

export const ProductionWithVanityUrl: Story = {
  args: {
    deployment: prodDeployment,
    deployments: allDeployments,
    whoseName: null,
    vanityUrl: "api.myapp.com",
  },
};

export const DevelopmentCloud: Story = {
  args: {
    deployment: devDeployment,
    deployments: allDeployments,
    whoseName: null,
  },
};

export const DevelopmentCloudTeammate: Story = {
  args: {
    deployment: devDeployment,
    deployments: allDeployments,
    whoseName: "Jane Doe",
  },
};

export const DevelopmentLocal: Story = {
  args: {
    deployment: localDevDeployment,
    deployments: allDeployments,
    whoseName: null,
  },
};

export const Preview: Story = {
  args: {
    deployment: previewDeployment,
    deployments: allDeployments,
    whoseName: null,
  },
};

export const Custom: Story = {
  args: {
    deployment: customDeployment,
    deployments: allDeployments,
    whoseName: null,
  },
};
