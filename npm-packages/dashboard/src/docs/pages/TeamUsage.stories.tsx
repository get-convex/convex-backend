import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
import {
  useUsageTeamSummary,
  useTokenUsage,
  useUsageTeamMetricsByFunction,
  useUsageTeamDailyCallsByTagByProject,
  useUsageTeamDatabaseBandwidthPerDayByProject,
  useUsageTeamDocumentsPerDayByProject,
  useUsageTeamDatabaseStoragePerDayByProject,
  useUsageTeamStoragePerDayByProject,
  useUsageTeamStorageThroughputDailyByProject,
  useUsageTeamVectorBandwidthPerDayByProject,
  useUsageTeamVectorStoragePerDayByProject,
  useUsageTeamDeploymentCountPerDayByProject,
  useUsageTeamDeploymentCountByType,
  useUsageTeamDatabaseStoragePerDayByTable,
  useUsageTeamDocumentCountPerDayByTable,
  useUsageTeamActionComputeDailyByProject,
} from "hooks/usageMetrics";
import { useCurrentBillingPeriod } from "api/usage";
import { TeamUsagePage } from "../../pages/t/[team]/settings/usage";

const meta = {
  component: TeamUsagePage,
  parameters: {
    layout: "fullscreen",
  },
  beforeEach: () => {
    mocked(useCurrentBillingPeriod).mockReturnValue({
      start: "2026-02-01",
      end: "2026-03-01",
    });
    mocked(useUsageTeamSummary).mockReturnValue({
      data: [
        {
          region: "aws-us-east-1",
          functionCalls: 5_000_000,
          actionCompute: 50,
          databaseStorage: 5_368_709_120,
          databaseBandwidth: 5_368_709_120,
          fileStorage: 10_737_418_240,
          fileBandwidth: 5_368_709_120,
          vectorStorage: 107_374_182,
          vectorBandwidth: 1_073_741_824,
        },
      ],
      error: undefined,
    });
    mocked(useTokenUsage).mockReturnValue({
      data: {
        centitokensUsed: 5_000_000,
        centitokensQuota: 2_500_000_000,
        tokensUsed: 50_000,
        tokensQuota: 25_000_000,
        isPaidPlan: true,
        isTeamDisabled: false,
        planType: "professional",
      },
      error: undefined,
      isLoading: false,
      mutate: async () => ({
        centitokensUsed: 5_000_000,
        centitokensQuota: 2_500_000_000,
        tokensUsed: 50_000,
        tokensQuota: 25_000_000,
        isPaidPlan: true,
        isTeamDisabled: false,
        planType: "professional",
      }),
      isValidating: false,
    });
    mocked(useUsageTeamMetricsByFunction).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDailyCallsByTagByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDatabaseBandwidthPerDayByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDocumentsPerDayByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDatabaseStoragePerDayByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamStoragePerDayByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamStorageThroughputDailyByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamVectorBandwidthPerDayByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamVectorStoragePerDayByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDeploymentCountPerDayByProject).mockReturnValue({
      data: [{ ds: "2026-02-28", projectId: 1, value: 3 }],
      error: undefined,
    });
    mocked(useUsageTeamDeploymentCountByType).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDatabaseStoragePerDayByTable).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamDocumentCountPerDayByTable).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useUsageTeamActionComputeDailyByProject).mockReturnValue({
      data: undefined,
      error: undefined,
    });
  },
} satisfies Meta<typeof TeamUsagePage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
