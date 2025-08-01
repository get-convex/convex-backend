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
    teamSummary: {
      databaseBandwidth: 4096 * 1024 * 1024,
      databaseStorage: 10 * 1024 * 1024,
      fileStorage: 15 * 1024,
      fileBandwidth: 10,
      functionCalls: 250000,
      actionCompute: 240,
      vectorBandwidth: 0,
      vectorStorage: 0,
    },
    entitlements: {
      teamMaxDatabaseBandwidth: 1024 * 1024 * 1024,
      teamMaxDatabaseStorage: 512 * 1024 * 1024, // 512 MB in bytes
      teamMaxFileBandwidth: 1024 * 1024 * 1024,
      teamMaxFileStorage: 1024 * 1024 * 1024,
      teamMaxFunctionCalls: 1000000,
      teamMaxActionCompute: 20,
      teamMaxVectorBandwidth: 512 * 1024 * 1024,
      teamMaxVectorStorage: 256 * 1024 * 1024,
      maxTeamMembers: 20,
      logStreamingEnabled: true,
      auditLogsEnabled: true,
      customDomainsEnabled: true,
      streamingExportEnabled: true,
      periodicBackupsEnabled: true,
      maxCloudBackups: 50,
      maxProjects: 10,
      projectMaxPreviewDeployments: 10,
      maxChefTokens: 8500000,
    },
    hasFilter: false,
  },
};
