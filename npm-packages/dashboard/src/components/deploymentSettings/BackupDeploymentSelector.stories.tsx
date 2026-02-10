import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked, fn } from "storybook/test";
import { TeamResponse, ProjectDetails, TeamMemberResponse } from "generatedApi";
import { useProfile } from "api/profile";
import { useProjects, useProjectById } from "api/projects";
import { useDeployments } from "api/deployments";
import { useTeamMembers } from "api/teams";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { BackupDeploymentSelector } from "./BackupDeploymentSelector";

const team: TeamResponse = {
  id: 1,
  creator: 1,
  slug: "test-team",
  name: "Test Team",
  suspended: false,
  referralCode: "TEAM123",
  referredBy: null,
};

const mockProjects: ProjectDetails[] = [
  {
    id: 1,
    name: "Project Alpha",
    slug: "project-alpha",
    teamId: 1,
    createTime: Date.now() - 30 * 24 * 60 * 60 * 1000,
    isDemo: false,
  },
  {
    id: 2,
    name: "Project Beta",
    slug: "project-beta",
    teamId: 1,
    createTime: Date.now() - 60 * 24 * 60 * 60 * 1000,
    isDemo: false,
  },
];

const mockTeamMembers: TeamMemberResponse[] = [
  {
    id: 1,
    email: "test@example.com",
    name: "Test User",
    role: "admin",
  },
  {
    id: 2,
    email: "jane@example.com",
    name: "Jane Doe",
    role: "developer",
  },
  {
    id: 3,
    email: "bob@example.com",
    name: "Bob Smith",
    role: "developer",
  },
];

const createDeployment = (overrides: {
  id: number;
  name: string;
  deploymentType: "prod" | "dev" | "preview" | "custom";
  creator?: number | null;
  isDefault?: boolean;
  previewIdentifier?: string | null;
}): PlatformDeploymentResponse => ({
  kind: "cloud",
  projectId: 1,
  creator: 1,
  createTime: Date.now() - 7 * 24 * 60 * 60 * 1000,
  region: "us-east-1",
  isDefault: false,
  previewIdentifier: null,
  ...overrides,
});

const mockDeployments: PlatformDeploymentResponse[] = [
  createDeployment({
    id: 1,
    name: "joyful-capybara-123",
    deploymentType: "prod",
    isDefault: true,
    creator: 1,
  }),
  createDeployment({
    id: 2,
    name: "happy-zebra-456",
    deploymentType: "dev",
    creator: 1, // Current user's dev deployment
  }),
  createDeployment({
    id: 3,
    name: "preview-feature-789",
    deploymentType: "preview",
    previewIdentifier: "feature/new-ui",
    creator: 1,
  }),
  createDeployment({
    id: 4,
    name: "custom-staging-321",
    deploymentType: "custom",
    creator: 1,
  }),
  createDeployment({
    id: 5,
    name: "jane-dev-deployment",
    deploymentType: "dev",
    creator: 2, // Jane's dev deployment
  }),
  createDeployment({
    id: 6,
    name: "bob-dev-deployment",
    deploymentType: "dev",
    creator: 3, // Bob's dev deployment
  }),
];

const meta = {
  component: BackupDeploymentSelector,
  beforeEach: () => {
    mocked(useProfile).mockReturnValue({
      id: 1,
      name: "Test User",
      email: "test@example.com",
    });
    mocked(useProjects).mockReturnValue(mockProjects);
    mocked(useDeployments).mockReturnValue({
      deployments: mockDeployments,
      isLoading: false,
    });
    mocked(useProjectById).mockReturnValue({
      project: mockProjects[0],
      isLoading: false,
      error: undefined,
    });
    mocked(useTeamMembers).mockReturnValue(mockTeamMembers);
  },
} satisfies Meta<typeof BackupDeploymentSelector>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    selectedDeployment: mockDeployments[0],
    targetDeployment: mockDeployments[0],
    team,
    onChange: fn(),
  },
};

export const DifferentDeploymentSelected: Story = {
  args: {
    selectedDeployment: mockDeployments[4], // Jane's dev deployment
    targetDeployment: mockDeployments[0],
    team,
    onChange: fn(),
  },
};
