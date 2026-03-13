import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import type { Meta, StoryObj } from "@storybook/nextjs";
import { MemberResponse } from "generatedApi";
import { DeploymentCard } from "./DeploymentCard";

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
    teamSlug: "my-team",
    teamMembers,
  },
  parameters: { a11y: { test: "todo" } },
} satisfies Meta<typeof DeploymentCard>;

export default meta;
type Story = StoryObj<typeof meta>;

// Production deployment
export const ProductionCloud: Story = {
  args: {
    deployment: {
      kind: "cloud",
      class: "s16",
      name: "happy-animal-123",
      deploymentType: "prod",
      createTime: Date.now() - 2 * 60 * 60 * 1000, // 2 hours ago
      creator: 1,
      id: 100,
      projectId: 1,
      isDefault: true,
      region: "aws-us-east-1",
    } as PlatformDeploymentResponse,
  },
};

// Development cloud deployment (mine)
export const DevelopmentCloudMine: Story = {
  args: {
    deployment: {
      kind: "cloud",
      class: "s16",
      name: "clever-otter-456",
      deploymentType: "dev",
      createTime: Date.now() - 30 * 60 * 1000, // 30 minutes ago
      creator: 1,
      id: 101,
      projectId: 1,
      isDefault: true,
      region: "aws-us-east-1",
    } as PlatformDeploymentResponse,
  },
};

// Development cloud deployment (teammate's)
export const DevelopmentCloudTeammate: Story = {
  args: {
    deployment: {
      kind: "cloud",
      class: "s16",
      name: "playful-koala-789",
      deploymentType: "dev",
      createTime: Date.now() - 4 * 60 * 60 * 1000, // 4 hours ago
      creator: 2,
      id: 102,
      projectId: 1,
      isDefault: false,
      region: "aws-us-east-1",
    } as PlatformDeploymentResponse,
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
      class: "s16",
      name: "preview-feature-123",
      deploymentType: "preview",
      createTime: Date.now() - 1 * 60 * 60 * 1000, // 1 hour ago
      creator: 1,
      id: 103,
      projectId: 1,
      isDefault: false,
      region: "aws-us-east-1",
      previewIdentifier: "feature-branch-xyz",
    } as PlatformDeploymentResponse,
  },
};

// Custom deployment
export const Custom: Story = {
  args: {
    deployment: {
      kind: "cloud",
      class: "s16",
      name: "custom-staging-789",
      deploymentType: "custom",
      createTime: Date.now() - 24 * 60 * 60 * 1000, // 1 day ago
      creator: 2,
      id: 104,
      projectId: 1,
      isDefault: false,
      region: "aws-eu-west-1",
    } as PlatformDeploymentResponse,
  },
};

// Old deployment (testing time display)
export const OldDeployment: Story = {
  args: {
    deployment: {
      kind: "cloud",
      class: "s16",
      name: "ancient-tortoise-111",
      deploymentType: "prod",
      createTime: Date.now() - 365 * 24 * 60 * 60 * 1000, // 1 year ago
      creator: 2,
      id: 107,
      projectId: 1,
      isDefault: true,
      region: "aws-eu-west-1",
    } as PlatformDeploymentResponse,
  },
};
