import { Meta, StoryObj } from "@storybook/nextjs";
import { PlanSummaryForTeam } from "./PlanSummary";

const meta = {
  component: PlanSummaryForTeam,
} satisfies Meta<typeof PlanSummaryForTeam>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    hasSubscription: true,
    showEntitlements: true,
    teamSummary: [
      {
        region: "aws-us-east-1",
        databaseBandwidth: 3072 * 1024 * 1024,
        databaseStorage: 8 * 1024 * 1024,
        fileStorage: 12 * 1024,
        fileBandwidth: 8,
        functionCalls: 200000,
        actionCompute: 180,
        vectorBandwidth: 0,
        vectorStorage: 0,
      },
      {
        region: "aws-eu-west-1",
        databaseBandwidth: 1024 * 1024 * 1024,
        databaseStorage: 2 * 1024 * 1024,
        fileStorage: 3 * 1024,
        fileBandwidth: 2,
        functionCalls: 50000,
        actionCompute: 60,
        vectorBandwidth: 0,
        vectorStorage: 0,
      },
    ],
    deploymentCount: 12,
    entitlements: {
      teamMaxDatabaseBandwidth: 1024 * 1024 * 1024,
      teamMaxDatabaseStorage: 512 * 1024 * 1024, // 512 MB in bytes
      teamMaxFileBandwidth: 1024 * 1024 * 1024,
      teamMaxFileStorage: 1024 * 1024 * 1024,
      teamMaxFunctionCalls: 1000000,
      teamMaxActionCompute: 20,
      teamMaxVectorBandwidth: 512 * 1024 * 1024,
      teamMaxVectorStorage: 256 * 1024 * 1024,
      maxTeamMembers: 50000,
      logStreamingEnabled: true,
      customDomainsEnabled: true,
      streamingExportEnabled: true,
      periodicBackupsEnabled: true,
      maxCloudBackups: 50,
      maxProjects: 10,
      maxChefTokens: 8500000,
      ssoEnabled: false,
      auditLogRetentionDays: 90,
      maxDeployments: 40,
      managementApiEnabled: true,
      previewDeploymentRetentionDays: 1,
      deploymentClassSelectionEnabled: false,
    },
    hasFilter: false,
  },
};
