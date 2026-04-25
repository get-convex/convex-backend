import { Meta, StoryObj } from "@storybook/nextjs";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import udfs from "@common/udfs";
import { ConvexProvider } from "convex/react";
import { fn, userEvent, waitFor, within } from "storybook/test";
import { IntegrationsView } from "@common/features/settings/components/integrations/IntegrationsView";

const mockTeam = { id: 2, slug: "acme", name: "Acme Corp" };
const mockProject = {
  id: 7,
  teamId: mockTeam.id,
  name: "My amazing app",
  slug: "my-amazing-app",
};
const mockDeployment = {
  id: 12,
  name: "musical-otter-456",
  deploymentType: "prod" as const,
  kind: "cloud",
  isDefault: true,
  projectId: mockProject.id,
  creator: 1,
  createTime: Date.now(),
  class: "s256",
  deploymentUrl: "https://musical-otter-456.eu-west-1.convex.cloud",
  reference: "production",
  region: "aws-eu-west-1",
} as const;

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listConfiguredSinks.default, () => [])
  .registerQueryFake(udfs.deploymentState.deploymentState, () => ({
    _id: "" as any,
    _creationTime: 0,
    state: "running" as const,
  }))
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getVersion.default, () => "1.18.0")
  .registerQueryFake(udfs.deploymentEvents.lastPushEvent, () => null)
  .registerQueryFake(
    udfs.convexCloudUrl.default,
    () => mockDeployment.deploymentUrl,
  )
  .registerQueryFake(
    udfs.convexSiteUrl.default,
    () => "https://musical-otter-456.convex.site",
  )
  .registerQueryFake(udfs.fileStorageV2.numFiles, () => 0)
  .registerQueryFake(udfs.tableSize.sizeOfAllTables, () => 0);

const mockConnectedDeployment = {
  deployment: {
    client: mockClient,
    httpClient: {} as never,
    deploymentUrl: mockDeployment.deploymentUrl,
    adminKey: "storybook-admin-key",
    deploymentName: mockDeployment.name,
  },
  isDisconnected: false,
};

const meta = {
  component: IntegrationsView,
  parameters: {
    layout: "fullscreen",
    docsPage: { deploymentType: "prod" },
    nextjs: {
      router: {
        pathname: "/t/[team]/[project]/[deploymentName]/settings/integrations",
        route: "/t/[team]/[project]/[deploymentName]/settings/integrations",
        asPath:
          "/t/acme/my-amazing-app/musical-otter-456/settings/integrations",
        query: {
          team: "acme",
          project: "my-amazing-app",
          deploymentName: "musical-otter-456",
        },
      },
    },
    a11y: { test: "todo" },
  },
  render: () => (
    <ConnectedDeploymentContext.Provider value={mockConnectedDeployment}>
      <ConvexProvider client={mockClient}>
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            useCurrentTeam: () => mockTeam,
            useCurrentProject: () => mockProject,
            useCurrentDeployment: () => mockDeployment,
            useIsDeploymentPaused: () => false,
            useLogDeploymentEvent: () => fn(),
            useTeamEntitlements: () => ({
              logStreamingEnabled: true,
              streamingExportEnabled: true,
            }),
            deploymentsURI: "/t/acme/my-amazing-app/musical-otter-456",
            projectsURI: "/t/acme/my-amazing-app",
            teamsURI: "/t/acme",
            isSelfHosted: false,
          }}
        >
          <IntegrationsView />
        </DeploymentInfoContext.Provider>
      </ConvexProvider>
    </ConnectedDeploymentContext.Provider>
  ),
} satisfies Meta<typeof IntegrationsView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

async function openConfigure(canvasElement: HTMLElement, label: string) {
  const canvas = within(canvasElement);
  await waitFor(() => canvas.getByText(label));
  const heading = canvas.getByText(label);
  const panel = heading.closest("div")!.parentElement!;
  await userEvent.click(within(panel).getByTestId("configure-integration"));
  await waitFor(() => {
    if (!document.querySelector('[data-testid="modal"]')) {
      throw new Error("modal not open yet");
    }
  });
}

export const ConfigureSentry: Story = {
  play: async ({ canvasElement }) => {
    await openConfigure(canvasElement, "Sentry");
  },
};

export const ConfigureDatadog: Story = {
  play: async ({ canvasElement }) => {
    await openConfigure(canvasElement, "Datadog");
  },
};
