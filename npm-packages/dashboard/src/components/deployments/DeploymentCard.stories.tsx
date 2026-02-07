import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import type { Meta, StoryObj } from "@storybook/nextjs";
import { ProjectDetails, MemberResponse } from "generatedApi";
import { DeploymentCard } from "./DeploymentCard";

const project: ProjectDetails = {
  id: 1,
  slug: "my-awesome-project",
  name: "My Awesome Project",
  teamId: 1,
  createTime: Date.now() - 30 * 24 * 60 * 60 * 1000, // 30 days ago
  isDemo: false,
};

const projectUntitled: ProjectDetails = {
  id: 2,
  slug: "untitled-project-123",
  name: "",
  teamId: 1,
  createTime: Date.now() - 7 * 24 * 60 * 60 * 1000, // 7 days ago
  isDemo: false,
};

const teamMembers: MemberResponse[] = [
  {
    id: 1,
    name: "Alice Johnson",
    email: "alice@convex.dev",
  },
  {
    id: 2,
    name: "Bob Smith",
    email: "bob@convex.dev",
  },
];

const meta = {
  component: DeploymentCard,
  args: {
    project,
    teamMembers,
    href: "#",
  },
} satisfies Meta<typeof DeploymentCard>;

export default meta;
type Story = StoryObj<typeof meta>;

// Production deployment
export const ProductionCloud: Story = {
  args: {
    deployment: {
      kind: "cloud",
      name: "happy-animal-123",
      deploymentType: "prod",
      createTime: Date.now() - 2 * 60 * 60 * 1000, // 2 hours ago
      creator: 1,
      id: 100,
      projectId: 1,
      isDefault: true,
      region: "us-east-1",
    } as PlatformDeploymentResponse,
  },
};

// Development cloud deployment (mine)
export const DevelopmentCloudMine: Story = {
  args: {
    deployment: {
      kind: "cloud",
      name: "clever-otter-456",
      deploymentType: "dev",
      createTime: Date.now() - 30 * 60 * 1000, // 30 minutes ago
      creator: 1,
      id: 101,
      projectId: 1,
      isDefault: true,
      region: "us-east-1",
    } as PlatformDeploymentResponse,
    whoseName: null, // null means it's mine
  },
};

// Development cloud deployment (teammate's)
export const DevelopmentCloudTeammate: Story = {
  args: {
    deployment: {
      kind: "cloud",
      name: "playful-koala-789",
      deploymentType: "dev",
      createTime: Date.now() - 4 * 60 * 60 * 1000, // 4 hours ago
      creator: 2,
      id: 102,
      projectId: 1,
      isDefault: false,
      region: "us-east-1",
    } as PlatformDeploymentResponse,
    whoseName: "Bob Smith",
  },
};

// Local development deployment
export const DevelopmentLocal: Story = {
  args: {
    deployment: {
      kind: "local",
      name: "local-dev",
      deploymentType: "dev",
      createTime: Date.now() - 15 * 60 * 1000, // 15 minutes ago
      creator: 1,
      projectId: 1,
      deviceName: "MacBook Pro",
      port: 3210,
      isActive: true,
    } as PlatformDeploymentResponse,
  },
};

// Preview deployment
export const Preview: Story = {
  args: {
    deployment: {
      kind: "cloud",
      name: "preview-feature-123",
      deploymentType: "preview",
      createTime: Date.now() - 1 * 60 * 60 * 1000, // 1 hour ago
      creator: 1,
      id: 103,
      projectId: 1,
      isDefault: false,
      region: "us-east-1",
      previewIdentifier: "feature-branch-xyz",
    } as PlatformDeploymentResponse,
  },
};

// Custom deployment
export const Custom: Story = {
  args: {
    deployment: {
      kind: "cloud",
      name: "custom-staging-789",
      deploymentType: "custom",
      createTime: Date.now() - 24 * 60 * 60 * 1000, // 1 day ago
      creator: 2,
      id: 104,
      projectId: 1,
      isDefault: false,
      region: "eu-west-1",
    } as PlatformDeploymentResponse,
  },
};

// Untitled project
export const UntitledProject: Story = {
  args: {
    project: projectUntitled,
    deployment: {
      kind: "cloud",
      name: "wonderful-panda-999",
      deploymentType: "prod",
      createTime: Date.now() - 3 * 60 * 60 * 1000, // 3 hours ago
      creator: 1,
      id: 105,
      projectId: 2,
      isDefault: true,
      region: "us-east-1",
    } as PlatformDeploymentResponse,
  },
};

// Long project name (testing truncation)
export const LongProjectName: Story = {
  args: {
    project: {
      ...project,
      name: "My Super Long Project Name That Should Truncate Nicely When Displayed",
      slug: "my-super-long-project-name-that-should-truncate",
    },
    deployment: {
      kind: "cloud",
      name: "happy-animal-123",
      deploymentType: "dev",
      createTime: Date.now() - 5 * 60 * 1000, // 5 minutes ago
      creator: 1,
      id: 106,
      projectId: 1,
      isDefault: true,
      region: "ap-southeast-2",
    } as PlatformDeploymentResponse,
  },
};

// Old deployment (testing time display)
export const OldDeployment: Story = {
  args: {
    deployment: {
      kind: "cloud",
      name: "ancient-tortoise-111",
      deploymentType: "prod",
      createTime: Date.now() - 365 * 24 * 60 * 60 * 1000, // 1 year ago
      creator: 2,
      id: 107,
      projectId: 1,
      isDefault: true,
      region: "us-west-2",
    } as PlatformDeploymentResponse,
  },
};
