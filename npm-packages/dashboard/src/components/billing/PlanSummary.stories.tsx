import { Meta, StoryObj } from "@storybook/react";
import { PlanSummaryForTeam } from "./PlanSummary";

export default {
  component: PlanSummaryForTeam,
} as Meta<typeof PlanSummaryForTeam>;

export const Primary: StoryObj<typeof PlanSummaryForTeam> = {
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
  },
};
