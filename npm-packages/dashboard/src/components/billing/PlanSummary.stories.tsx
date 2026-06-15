import { Meta, StoryObj } from "@storybook/nextjs";
import { BusinessPlanSummary } from "./PlanSummary";

const meta = {
  component: BusinessPlanSummary,
  parameters: { a11y: { test: "todo" } },
} satisfies Meta<typeof BusinessPlanSummary>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    hasSubscription: true,
    showEntitlements: true,
    isBusinessPlan: false,
    summary: [
      {
        deploymentClass: "s16",
        region: "aws-us-east-1",
        databaseStorage: 8 * 1024 * 1024,
        databaseIO: 3072 * 1024 * 1024,
        functionCalls: 200000,
        queryMutationCompute: 60,
        actionComputeConvex: 120,
        actionComputeNode: 60,
        actionComputeUser: 120,
        fileStorage: 12 * 1024,
        searchStorage: 0,
        dataEgress: 8,
        searchQueries: 0,
      },
      {
        deploymentClass: "s16",
        region: "aws-eu-west-1",
        databaseStorage: 2 * 1024 * 1024,
        databaseIO: 1024 * 1024 * 1024,
        functionCalls: 50000,
        queryMutationCompute: 20,
        actionComputeConvex: 40,
        actionComputeNode: 20,
        actionComputeUser: 40,
        fileStorage: 3 * 1024,
        searchStorage: 0,
        dataEgress: 2,
        searchQueries: 0,
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
      teamMaxSearchQueries: 50000,
      teamMaxVectorBandwidth: 512 * 1024 * 1024,
      teamMaxVectorStorage: 256 * 1024 * 1024,
      maxTeamMembers: 50000,
      logStreamingEnabled: true,
      customDomainsEnabled: true,
      customRolesEnabled: true,
      customAuditLogsInLogStreamsConfigEnabled: true,
      streamingExportEnabled: true,
      periodicBackupsEnabled: true,
      maxCloudBackups: 50,
      maxChefTokens: 8500000,
      ssoEnabled: false,
      auditLogRetentionDays: 90,
      maxDeployments: 40,
      managementApiEnabled: true,
      previewDeploymentRetentionDays: 1,
      deploymentClassSelectionEnabled: false,
    },
  },
};
