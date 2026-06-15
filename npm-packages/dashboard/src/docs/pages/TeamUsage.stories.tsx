import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
import {
  useUsageTeamDocumentsPerDayByProject,
  useUsageTeamDeploymentCountPerDayByProject,
  useUsageTeamDeploymentCountByType,
} from "hooks/usageMetricsV2";
import {
  useUsageTeamSummaryV2,
  useUsageTeamMetricsByFunctionV2,
  useDatabaseStoragePerDayByProjectAndClassV2,
  useDatabaseStoragePerDayByTableV2,
  useDocumentCountPerDayByTableV2,
  useDatabaseIOPerDayByProjectAndClassV2,
  useFunctionCallsPerDayByProjectAndClassV2,
  useComputePerDayByProjectV2,
  useFileStoragePerDayByProjectV2,
  useSearchStoragePerDayByProjectV2,
  useDataEgressPerDayByProjectV2,
  useSearchQueriesPerDayByProjectV2,
  useDeploymentsByClassAndRegionV2,
} from "hooks/usageMetricsV2";
import { useCurrentBillingPeriod } from "api/usage";
import { TeamUsagePage } from "../../pages/t/[team]/settings/usage";

const meta = {
  component: TeamUsagePage,
  parameters: {
    layout: "fullscreen",
    a11y: {
      test: "todo",
    },
  },
  beforeEach: () => {
    mocked(useCurrentBillingPeriod).mockReturnValue({
      start: "2026-02-01",
      end: "2026-03-01",
    });
    mocked(useUsageTeamDocumentsPerDayByProject).mockReturnValue({
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
    mocked(useUsageTeamSummaryV2).mockReturnValue({
      data: [
        {
          deploymentClass: "s16",
          region: "aws-us-east-1",
          databaseStorage: 5_368_709_120,
          databaseIO: 5_368_709_120,
          functionCalls: 5_000_000,
          queryMutationCompute: 10,
          actionComputeConvex: 30,
          actionComputeNode: 20,
          fileStorage: 10_737_418_240,
          searchStorage: 107_374_182,
          dataEgress: 5_368_709_120,
          searchQueries: 500,
          actionComputeUser: 30,
        },
      ],
      error: undefined,
    });
    mocked(useUsageTeamMetricsByFunctionV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useDatabaseStoragePerDayByProjectAndClassV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useDatabaseStoragePerDayByTableV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useDocumentCountPerDayByTableV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useDatabaseIOPerDayByProjectAndClassV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useFunctionCallsPerDayByProjectAndClassV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useComputePerDayByProjectV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useFileStoragePerDayByProjectV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useSearchStoragePerDayByProjectV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useDataEgressPerDayByProjectV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useSearchQueriesPerDayByProjectV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
    mocked(useDeploymentsByClassAndRegionV2).mockReturnValue({
      data: undefined,
      error: undefined,
    });
  },
} satisfies Meta<typeof TeamUsagePage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
