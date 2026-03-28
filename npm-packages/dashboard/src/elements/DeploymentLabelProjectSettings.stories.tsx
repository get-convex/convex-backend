import { DeploymentLabelProjectSettings } from "./DeploymentDisplay";
import { type Meta, type StoryObj } from "@storybook/nextjs";
import {
  ProjectDetails,
  TeamResponse,
  PlatformDeploymentResponse,
} from "generatedApi";

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

const deployments: PlatformDeploymentResponse[] = [];

const meta = {
  component: DeploymentLabelProjectSettings,
  parameters: { a11y: { test: "todo" } },
} satisfies Meta<typeof DeploymentLabelProjectSettings>;

export default meta;
type Story = StoryObj<typeof DeploymentLabelProjectSettings>;

export const Default: Story = {
  args: {
    team: mockTeam,
    currentProject: mockProject,
    deployments,
  },
};
