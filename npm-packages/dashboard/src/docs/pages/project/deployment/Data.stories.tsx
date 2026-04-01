import { Meta, StoryObj } from "@storybook/nextjs";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import udfs from "@common/udfs";
import { ConvexProvider } from "convex/react";
import { fn, mocked } from "storybook/test";
import { useTableShapes } from "@common/lib/deploymentApi";
import { Shape } from "shapes";
import { DataView } from "@common/features/data/components/DataView";
import { FunctionsContext } from "@common/lib/functions/FunctionsProvider";
import { api } from "system-udfs/convex/_generated/api";

const mockTeam = {
  id: 2,
  slug: "acme",
  name: "Acme Corp",
};

const mockProject = {
  id: 7,
  teamId: mockTeam.id,
  name: "My amazing app",
  slug: "my-amazing-app",
};

const mockDeployment = {
  id: 11,
  name: "happy-capybara-123",
  deploymentType: "dev" as const,
  kind: "cloud",
  isDefault: true,
  projectId: mockProject.id,
  creator: 1,
  createTime: Date.now(),
  class: "s256",
  deploymentUrl: "https://happy-capybara-123.convex.cloud",
  reference: "dev/nicolas",
  region: "aws-us-east-1",
} as const;

const mockShapesData: Record<string, Shape> = {
  channels: {
    type: "Object",
    fields: [
      {
        fieldName: "_id",
        optional: false,
        shape: { type: "Id", tableName: "channels" },
      },
      {
        fieldName: "_creationTime",
        optional: false,
        shape: { type: "Float64", float64Range: {} },
      },
      { fieldName: "name", optional: false, shape: { type: "String" } },
      { fieldName: "description", optional: true, shape: { type: "String" } },
    ],
  },
  messages: {
    type: "Object",
    fields: [
      {
        fieldName: "_id",
        optional: false,
        shape: { type: "Id", tableName: "messages" },
      },
      {
        fieldName: "_creationTime",
        optional: false,
        shape: { type: "Float64", float64Range: {} },
      },
      { fieldName: "body", optional: false, shape: { type: "String" } },
      {
        fieldName: "author",
        optional: false,
        shape: { type: "Id", tableName: "users" },
      },
      {
        fieldName: "channel",
        optional: false,
        shape: { type: "Id", tableName: "channels" },
      },
    ],
  },
  users: {
    type: "Object",
    fields: [
      {
        fieldName: "_id",
        optional: false,
        shape: { type: "Id", tableName: "users" },
      },
      {
        fieldName: "_creationTime",
        optional: false,
        shape: { type: "Float64", float64Range: {} },
      },
      { fieldName: "name", optional: false, shape: { type: "String" } },
      { fieldName: "email", optional: false, shape: { type: "String" } },
      {
        fieldName: "isAdmin",
        optional: true,
        shape: { type: "Boolean" },
      },
    ],
  },
};

const now = Date.now();

const mockDocuments: Record<string, any[]> = {
  channels: [
    {
      _id: "k17cx2tgs5e3ah8b0gnkq9de2s6yxr3w" as any,
      _creationTime: now - 100000,
      name: "general",
      description: "General discussion",
    },
    {
      _id: "k17d83nrq4a7pm5g1fhev0jb9t6zxc2k" as any,
      _creationTime: now - 90000,
      name: "engineering",
      description: "Engineering team",
    },
    {
      _id: "k17eqw4hy8b2vn6d3jrms0fa1t5gxp7z" as any,
      _creationTime: now - 80000,
      name: "random",
    },
  ],
  messages: [
    {
      _id: "k27fv9xgn3c4wk8e2dpht1ja6m5byr0q" as any,
      _creationTime: now - 50000,
      body: "Hello, world!",
      author: "k37ax5mrd2e8tn4f1ghqv0jb7w6ypc9k",
      channel: "k17cx2tgs5e3ah8b0gnkq9de2s6yxr3w",
    },
    {
      _id: "k27gm6bpz1d9xr3h5eqwt4ka8n7fyc2j" as any,
      _creationTime: now - 40000,
      body: "Welcome to our app",
      author: "k37bh2nqe7f4yk9g3jtmv1da6w8xpc5r",
      channel: "k17cx2tgs5e3ah8b0gnkq9de2s6yxr3w",
    },
    {
      _id: "k27hn3ctw8e5yp6j2frvx1kb4m9gqd7a" as any,
      _creationTime: now - 30000,
      body: "Great work on the new feature!",
      author: "k37ax5mrd2e8tn4f1ghqv0jb7w6ypc9k",
      channel: "k17d83nrq4a7pm5g1fhev0jb9t6zxc2k",
    },
  ],
  users: [
    {
      _id: "k37ax5mrd2e8tn4f1ghqv0jb7w6ypc9k" as any,
      _creationTime: now - 200000,
      name: "Alice Johnson",
      email: "alice@example.com",
      isAdmin: true,
    },
    {
      _id: "k37bh2nqe7f4yk9g3jtmv1da6w8xpc5r" as any,
      _creationTime: now - 190000,
      name: "Bob Smith",
      email: "bob@example.com",
    },
  ],
};

const mockConvexClient = mockConvexReactClient()
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getVersion.default, () => "1.18.0")
  .registerQueryFake(udfs.getSchemas.default, () => ({
    active: undefined,
    inProgress: undefined,
  }))
  .registerQueryFake(udfs.getSchemas.schemaValidationProgress, () => null)
  .registerQueryFake(
    udfs.tableSize.default,
    ({ tableName }: { tableName: string }) =>
      mockDocuments[tableName]?.length ?? 0,
  )
  .registerQueryFake(
    udfs.paginatedTableDocuments.default,
    ({
      table,
    }: {
      table: string;
      paginationOpts: any;
      filters: string | null;
    }) => ({
      page: mockDocuments[table] ?? [],
      isDone: true,
      continueCursor: "",
    }),
  )
  .registerQueryFake(api._system.frontend.indexes.default, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({
    1: "channels",
    2: "messages",
    3: "users",
  }))
  .registerQueryFake(udfs.deploymentState.deploymentState, () => ({
    _id: "" as any,
    _creationTime: 0,
    state: "running" as const,
  }))
  .registerQueryFake(udfs.deploymentEvents.lastPushEvent, () => null)
  .registerQueryFake(
    udfs.convexCloudUrl.default,
    () => mockDeployment.deploymentUrl,
  )
  .registerQueryFake(
    udfs.convexSiteUrl.default,
    () => "https://happy-capybara-123.convex.site",
  )
  .registerQueryFake(udfs.fileStorageV2.numFiles, () => 0)
  .registerQueryFake(udfs.tableSize.sizeOfAllTables, () => 0);

const mockConnectedDeployment = {
  deployment: {
    client: mockConvexClient,
    httpClient: {} as never,
    deploymentUrl: mockDeployment.deploymentUrl,
    adminKey: "storybook-admin-key",
    deploymentName: mockDeployment.name,
  },
  isDisconnected: false,
};

const meta = {
  component: DataView,
  parameters: {
    layout: "fullscreen",
    nextjs: {
      router: {
        pathname: "/t/[team]/[project]/[deploymentName]/data",
        route: "/t/[team]/[project]/[deploymentName]/data",
        asPath: "/t/acme/my-amazing-app/happy-capybara-123/data",
        query: {
          team: "acme",
          project: "my-amazing-app",
          deploymentName: "happy-capybara-123",
        },
      },
    },
    a11y: { test: "todo" },
  },
  decorators: [
    (storyFn) => {
      const tables = new Map(
        Object.entries(mockShapesData)
          .sort()
          .filter(([name]) => !name.startsWith("_")),
      );
      mocked(useTableShapes).mockReturnValue({ tables, hadError: false });
      return storyFn();
    },
  ],
  render: () => (
    <ConnectedDeploymentContext.Provider value={mockConnectedDeployment}>
      <ConvexProvider client={mockConvexClient}>
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            useCurrentTeam: () => mockTeam,
            useCurrentProject: () => mockProject,
            useCurrentDeployment: () => mockDeployment,
            useIsDeploymentPaused: () => false,
            useLogDeploymentEvent: () => fn(),
            deploymentsURI: "/t/acme/my-amazing-app/happy-capybara-123",
            projectsURI: "/t/acme/my-amazing-app",
            teamsURI: "/t/acme",
            isSelfHosted: false,
          }}
        >
          <FunctionsContext.Provider value={new Map()}>
            <DataView />
          </FunctionsContext.Provider>
        </DeploymentInfoContext.Provider>
      </ConvexProvider>
    </ConnectedDeploymentContext.Provider>
  ),
} satisfies Meta<typeof DataView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
