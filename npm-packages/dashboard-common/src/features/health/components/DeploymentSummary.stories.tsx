import type { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { DeploymentSummary } from "./DeploymentSummary";

const mockLastPushEvent = {
  _id: "123" as any,
  _creationTime: Date.now() - 1000 * 60 * 30, // 30 minutes ago
  member_id: BigInt(123),
  action: "push_config" as const,
  metadata: {} as any,
} as any;

const mockConvexCloudUrl = "https://happy-animal-123.convex.cloud";
const mockConvexSiteUrl = "https://happy-animal-123.convex.site";

const mockClient = mockConvexReactClient()
  .registerQueryFake(
    udfs.deploymentEvents.lastPushEvent,
    () => mockLastPushEvent,
  )
  .registerQueryFake(udfs.convexCloudUrl.default, () => mockConvexCloudUrl)
  .registerQueryFake(udfs.convexSiteUrl.default, () => mockConvexSiteUrl)
  .registerQueryFake(udfs.getVersion.default, () => "1.18.0");

const mockClientNeverDeployed = mockConvexReactClient()
  .registerQueryFake(udfs.deploymentEvents.lastPushEvent, () => null)
  .registerQueryFake(udfs.convexCloudUrl.default, () => mockConvexCloudUrl)
  .registerQueryFake(udfs.convexSiteUrl.default, () => mockConvexSiteUrl)
  .registerQueryFake(udfs.getVersion.default, () => "1.18.0");

const prodDeployment: PlatformDeploymentResponse = {
  id: 1,
  name: "happy-animal-123",
  deploymentType: "prod",
  projectId: 1,
  kind: "cloud",
  region: "us-east-1",
  createTime: Date.now() - 1000 * 60 * 60 * 24 * 7, // 1 week ago
  isDefault: true,
  creator: null,
  previewIdentifier: null,
  reference: "production",
};

const devCloudDeployment: PlatformDeploymentResponse = {
  id: 2,
  name: "joyful-capybara-123",
  deploymentType: "dev",
  projectId: 1,
  kind: "cloud",
  region: "us-west-2",
  createTime: Date.now() - 1000 * 60 * 60 * 24 * 2, // 2 days ago
  isDefault: false,
  creator: 123,
  previewIdentifier: null,
  reference: "dev/nicolas",
};

const devLocalDeployment: PlatformDeploymentResponse = {
  name: "local-dev-789",
  deploymentType: "dev",
  projectId: 1,
  kind: "local",
  createTime: Date.now() - 1000 * 60 * 60, // 1 hour ago
  creator: 123,
  deviceName: "MacBook Pro",
  port: 3210,
  isActive: true,
  previewIdentifier: null,
};

const previewDeployment: PlatformDeploymentResponse = {
  id: 4,
  name: "musical-dog-123",
  deploymentType: "preview",
  projectId: 1,
  kind: "cloud",
  region: "eu-west-1",
  createTime: Date.now() - 1000 * 60 * 60 * 12, // 12 hours ago
  isDefault: false,
  creator: 456,
  previewIdentifier: "pr-42",
  reference: "preview/my-feature",
};

const customDeployment: PlatformDeploymentResponse = {
  id: 5,
  name: "wandering-fish-513",
  deploymentType: "custom",
  projectId: 1,
  kind: "cloud",
  region: "ap-southeast-1",
  createTime: Date.now() - 1000 * 60 * 60 * 24 * 30, // 30 days ago
  isDefault: false,
  creator: 789,
  previewIdentifier: null,
  reference: "staging",
};

const mockRegions = [
  { name: "us-east-1", displayName: "US East (N. Virginia)" },
  { name: "us-west-2", displayName: "US West (Oregon)" },
  { name: "eu-west-1", displayName: "EU (Ireland)" },
  { name: "ap-southeast-1", displayName: "Asia Pacific (Singapore)" },
];

const meta = {
  component: DeploymentSummary,
  args: {
    deployment: prodDeployment,
    teamSlug: "my-team",
    projectSlug: "my-project",
    lastBackupTime: Date.now() - 1000 * 60 * 60 * 24, // 1 day ago
    creatorId: 123,
    creatorName: "Ari",
    regions: mockRegions,
  },
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <div className="max-w-4xl">
          <DeploymentSummary {...args} />
        </div>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} satisfies Meta<typeof DeploymentSummary>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Production: Story = {};

export const ProductionWithRecentBackup: Story = {
  args: {
    lastBackupTime: Date.now() - 1000 * 60 * 15, // 15 minutes ago
  },
};

export const ProductionNoBackups: Story = {
  args: {
    lastBackupTime: null,
  },
};

export const ProductionNeverDeployed: Story = {
  render: (args) => (
    <ConvexProvider client={mockClientNeverDeployed}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <div className="max-w-4xl">
          <DeploymentSummary {...args} />
        </div>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
};

export const DevelopmentCloud: Story = {
  args: {
    deployment: devCloudDeployment,
  },
};

export const DevelopmentLocal: Story = {
  args: {
    deployment: devLocalDeployment,
  },
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <div className="max-w-4xl">
          <DeploymentSummary {...args} />
        </div>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
};

export const Preview: Story = {
  args: {
    deployment: previewDeployment,
  },
};

export const Custom: Story = {
  args: {
    deployment: customDeployment,
  },
};

export const Loading: Story = {
  render: (args) => (
    <ConvexProvider
      client={mockConvexReactClient()
        .registerQueryFake(
          udfs.deploymentEvents.lastPushEvent,
          () => undefined as any,
        )
        .registerQueryFake(udfs.convexCloudUrl.default, () => undefined as any)
        .registerQueryFake(udfs.convexSiteUrl.default, () => undefined as any)
        .registerQueryFake(udfs.getVersion.default, () => undefined as any)}
    >
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <div className="max-w-4xl">
          <DeploymentSummary {...args} lastBackupTime={undefined} />
        </div>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
};
