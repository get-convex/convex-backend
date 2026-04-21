import { Meta, StoryObj } from "@storybook/nextjs";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import udfs from "@common/udfs";
import { ConvexProvider } from "convex/react";
import { GenericId } from "convex/values";
import { GenericDocument } from "convex/server";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { useDeployments } from "api/deployments";
import { fn, mocked, userEvent, within, waitFor, expect } from "storybook/test";
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

const mockDocuments: Record<string, GenericDocument[]> = {
  channels: [
    {
      _id: "k17cx2tgs5e3ah8b0gnkq9de2s6yxr3w" as GenericId<string>,
      _creationTime: now - 100000,
      name: "general",
      description: "General discussion",
    },
    {
      _id: "k17d83nrq4a7pm5g1fhev0jb9t6zxc2k" as GenericId<string>,
      _creationTime: now - 90000,
      name: "engineering",
      description: "Engineering team",
    },
    {
      _id: "k17eqw4hy8b2vn6d3jrms0fa1t5gxp7z" as GenericId<string>,
      _creationTime: now - 80000,
      name: "random",
    },
  ],
  messages: [
    {
      _id: "k27fv9xgn3c4wk8e2dpht1ja6m5byr0q" as GenericId<string>,
      _creationTime: now - 50000,
      body: "Hello, world!",
      author: "k37ax5mrd2e8tn4f1ghqv0jb7w6ypc9k",
      channel: "k17cx2tgs5e3ah8b0gnkq9de2s6yxr3w",
    },
    {
      _id: "k27gm6bpz1d9xr3h5eqwt4ka8n7fyc2j" as GenericId<string>,
      _creationTime: now - 40000,
      body: "Welcome to our app",
      author: "k37bh2nqe7f4yk9g3jtmv1da6w8xpc5r",
      channel: "k17cx2tgs5e3ah8b0gnkq9de2s6yxr3w",
    },
    {
      _id: "k27hn3ctw8e5yp6j2frvx1kb4m9gqd7a" as GenericId<string>,
      _creationTime: now - 30000,
      body: "Great work on the new feature!",
      author: "k37ax5mrd2e8tn4f1ghqv0jb7w6ypc9k",
      channel: "k17d83nrq4a7pm5g1fhev0jb9t6zxc2k",
    },
  ],
  users: [
    {
      _id: "k37ax5mrd2e8tn4f1ghqv0jb7w6ypc9k" as GenericId<string>,
      _creationTime: now - 200000,
      name: "Alice Johnson",
      email: "alice@example.com",
      isAdmin: true,
    },
    {
      _id: "k37bh2nqe7f4yk9g3jtmv1da6w8xpc5r" as GenericId<string>,
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
    ({ table }: { table: string }) => ({
      page: mockDocuments[table] ?? [],
      isDone: true,
      continueCursor: "",
    }),
  )
  .registerQueryFake(api._system.frontend.indexes.default, () => [
    {
      name: "by_name",
      fields: ["name", "_creation_time"],
      staged: false,
      backfill: { state: "done" as const },
    },
  ])
  .registerQueryFake(udfs.getTableMapping.default, () => ({
    1: "channels",
    2: "messages",
    3: "users",
  }))
  .registerQueryFake(udfs.deploymentState.deploymentState, () => ({
    _id: "" as GenericId<"_backend_state">,
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

/**
 * Shows the Data page with filter panel open, sorting by the `by_name` index.
 */
export const Filters: Story = {
  parameters: {
    nextjs: {
      router: {
        pathname: "/t/[team]/[project]/[deploymentName]/data",
        route: "/t/[team]/[project]/[deploymentName]/data",
        asPath:
          "/t/acme/my-amazing-app/happy-capybara-123/data?filters=eyJjbGF1c2VzIjpbXSwiaW5kZXgiOnsibmFtZSI6ImJ5X25hbWUiLCJjbGF1c2VzIjpbeyJ0eXBlIjoiaW5kZXhFcSIsImVuYWJsZWQiOnRydWUsInZhbHVlIjoiZ2VuZXJhbCJ9LHsidHlwZSI6ImluZGV4RXEiLCJlbmFibGVkIjpmYWxzZSwidmFsdWUiOjE3NzU1MTUxNjk5NDd9XX19",
        query: {
          team: "acme",
          project: "my-amazing-app",
          deploymentName: "happy-capybara-123",
          filters:
            "eyJjbGF1c2VzIjpbXSwiaW5kZXgiOnsibmFtZSI6ImJ5X25hbWUiLCJjbGF1c2VzIjpbeyJ0eXBlIjoiaW5kZXhFcSIsImVuYWJsZWQiOnRydWUsInZhbHVlIjoiZ2VuZXJhbCJ9LHsidHlwZSI6ImluZGV4RXEiLCJlbmFibGVkIjpmYWxzZSwidmFsdWUiOjE3NzU1MTUxNjk5NDd9XX19",
        },
      },
    },
    screenshotSelector: '[data-testid="filterMenu"]',
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the Filter button to appear and click it to open the filter panel
    await waitFor(
      async () => {
        await expect(canvas.queryByLabelText("Filter")).toBeTruthy();
      },
      { timeout: 5000 },
    );
    await userEvent.click(canvas.getByLabelText("Filter"));
  },
};

/**
 * Shows the Data page with "Add documents" panel open.
 */
export const AddDocument: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the toolbar to be visible
    await waitFor(async () => {
      await expect(
        canvas.getByRole("button", { name: /add|import/i }),
      ).toBeDefined();
    });

    // Click the "Add" button
    const addButton = canvas.getByRole("button", { name: /add/i });
    await userEvent.click(addButton);

    // Wait for the add documents panel to appear
    await waitFor(async () => {
      const inputs = canvas.queryAllByRole("textbox");
      await expect(inputs.length).toBeGreaterThan(0);
    });

    // Wait for Monaco editor to fully render and stabilize
    await new Promise((r) => setTimeout(r, 5000));
  },
};

/**
 * Shows inline cell editor when double-clicking a cell.
 */
export const EditInline: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the table to be visible with data cells
    let cellButton: HTMLElement | null = null;
    await waitFor(async () => {
      const buttons = canvas.queryAllByTestId("cell-editor-button");
      cellButton =
        buttons.find((btn) =>
          btn.textContent?.includes("General discussion"),
        ) ?? null;
      await expect(cellButton).toBeTruthy();
    });

    // Double-click the cell button to open the inline editor
    if (cellButton) {
      await userEvent.dblClick(cellButton);
    }

    // Cell editor popper renders in a portal at document.body
    const body = within(document.body);
    await waitFor(async () => {
      await expect(body.queryByTestId("cell-editor-popper")).toBeTruthy();
    });
  },
  parameters: {
    screenshotSelector: '[data-testid="cell-editor-popper"]',
  },
};

/**
 * Shows context menu when right-clicking a table cell.
 */
export const EditDocument: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for cells to render
    let cellButton: HTMLElement | null = null;
    await waitFor(async () => {
      const buttons = canvas.queryAllByTestId("cell-editor-button");
      cellButton =
        buttons.find((btn) =>
          btn.textContent?.includes("General discussion"),
        ) ?? null;
      await expect(cellButton).toBeTruthy();
    });

    // Dispatch a native contextmenu event (the context menu listens via addEventListener)
    if (cellButton) {
      const rect = (cellButton as HTMLElement).getBoundingClientRect();
      (cellButton as HTMLElement).dispatchEvent(
        new MouseEvent("contextmenu", {
          bubbles: true,
          clientX: rect.left + rect.width / 2,
          clientY: rect.top + rect.height / 2,
        }),
      );
    }

    // Context menu renders in a FloatingPortal, query from document.body
    const body = within(document.body);
    await waitFor(async () => {
      await expect(body.queryByTestId("table-context-menu")).toBeTruthy();
    });

    // Navigate to "Edit Document" using native KeyboardEvents on the menu.
    // The ContextMenu uses Floating UI's useListNavigation which tracks
    // activeIndex via keyboard events on the menu's role="menu" element.
    // Menu items: View desc, Copy desc, Edit desc, Filter by desc,
    //             View Doc, Copy Doc, Edit Doc, Delete Doc
    // 7 ArrowDown presses from null activeIndex to reach "Edit Document".
    const menuEl = body.getByRole("menu");
    menuEl.focus();
    for (let i = 0; i < 7; i++) {
      menuEl.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "ArrowDown",
          bubbles: true,
        }),
      );
      // Small delay to let React process each navigation step
      await new Promise((r) => setTimeout(r, 50));
    }
  },
  parameters: {
    screenshotSelector: '[data-testid="table-context-menu"]',
  },
};

/**
 * Shows context menu with expanded submenu.
 */
export const ContextMenu: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for cells to render
    let cellButton: HTMLElement | null = null;
    await waitFor(async () => {
      const buttons = canvas.queryAllByTestId("cell-editor-button");
      cellButton =
        buttons.find((btn) =>
          btn.textContent?.includes("General discussion"),
        ) ?? null;
      await expect(cellButton).toBeTruthy();
    });

    // Dispatch a native contextmenu event
    if (cellButton) {
      const rect = (cellButton as HTMLElement).getBoundingClientRect();
      (cellButton as HTMLElement).dispatchEvent(
        new MouseEvent("contextmenu", {
          bubbles: true,
          clientX: rect.left + rect.width / 2,
          clientY: rect.top + rect.height / 2,
        }),
      );
    }

    // Context menu renders in a FloatingPortal, query from document.body
    const body = within(document.body);
    await waitFor(async () => {
      await expect(body.queryByTestId("table-context-menu")).toBeTruthy();
    });

    // Hover over "Filter by" submenu item to expand it
    const contextMenu = body.getByTestId("table-context-menu");
    const filterByItem = within(contextMenu).queryByRole("menuitem", {
      name: /filter by/i,
    });
    if (filterByItem) {
      await userEvent.hover(filterByItem);
    }
  },
  parameters: {
    screenshotSelector: '[role="menu"]',
  },
};

/**
 * Shows Data page with 2 rows selected via checkboxes.
 */
export const BulkEdit: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the table to be visible
    await waitFor(async () => {
      const checkboxes = canvas.queryAllByRole("checkbox");
      await expect(checkboxes.length).toBeGreaterThan(0);
    });

    // Click the first row checkbox
    const checkboxes = canvas.getAllByRole("checkbox");
    if (checkboxes.length > 0) {
      await userEvent.click(checkboxes[0]);
    }

    // Click the second row checkbox
    if (checkboxes.length > 1) {
      await userEvent.click(checkboxes[1]);
    }

    // Wait to ensure rows are selected
    await waitFor(async () => {
      const selectedCheckboxes = canvas.queryAllByRole("checkbox", {
        checked: true,
      });
      await expect(selectedCheckboxes.length).toBeGreaterThanOrEqual(2);
    });
  },
};

/**
 * Shows Data page with overflow menu (three-dot menu) open.
 */
export const CustomQuery: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the overflow menu button to be visible
    await waitFor(async () => {
      await expect(canvas.queryByLabelText("Open table settings")).toBeTruthy();
    });

    // Click the overflow menu button (aria-label="Open table settings")
    const overflowButton = canvas.getByLabelText("Open table settings");
    await userEvent.click(overflowButton);

    // Menu items render in a Headless UI Portal, query from document.body
    const body = within(document.body);
    await waitFor(async () => {
      await expect(body.queryAllByRole("menuitem").length).toBeGreaterThan(0);
    });
  },
  parameters: {
    screenshotSelector: '[role="menu"]',
  },
};

/**
 * Shows Data page with overflow menu open and "Custom query" item highlighted.
 */
export const CustomQueryRunner: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the overflow menu button to be visible
    await waitFor(async () => {
      await expect(canvas.queryByLabelText("Open table settings")).toBeTruthy();
    });

    // Click the overflow menu button
    const overflowButton = canvas.getByLabelText("Open table settings");
    await userEvent.click(overflowButton);

    // Menu items render in a Headless UI Portal, query from document.body
    const body = within(document.body);
    await waitFor(async () => {
      await expect(body.queryAllByRole("menuitem").length).toBeGreaterThan(0);
    });

    // Click on "Custom query" menu item
    const customQueryItem = body.queryByRole("menuitem", {
      name: /custom query/i,
    });
    if (customQueryItem) {
      await userEvent.click(customQueryItem);
    }
  },
};

/**
 * Shows Data page with the full schema view open.
 */
export const GenerateSchema: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Wait for the sidebar "Schema" button to be visible
    await waitFor(async () => {
      await expect(
        canvas.queryByRole("button", { name: /^Schema$/ }),
      ).toBeTruthy();
    });

    // Click the sidebar "Schema" button
    const schemaButton = canvas.getByRole("button", { name: /^Schema$/ });
    await userEvent.click(schemaButton);
  },
};

export const MultipleDevDeploymentsSelector: Story = {
  parameters: {
    ...meta.parameters,
    screenshotSelector: "#select-deployment, [role='menu']",
  },
  decorators: [
    (storyFn) => {
      mocked(useDeployments).mockReturnValue({
        deployments: [
          {
            id: 11,
            name: "happy-capybara-123",
            deploymentType: "dev" as const,
            kind: "cloud" as const,
            isDefault: true,
            projectId: mockProject.id,
            creator: 1,
            createTime: Date.now(),
            class: "s256",
            deploymentUrl: "https://happy-capybara-123.convex.cloud",
            reference: "dev/nicolas",
            region: "aws-us-east-1",
          },
          {
            id: 12,
            name: "musical-otter-456",
            deploymentType: "prod" as const,
            kind: "cloud" as const,
            isDefault: true,
            projectId: mockProject.id,
            creator: 1,
            createTime: Date.now(),
            class: "s256",
            deploymentUrl: "https://musical-otter-456.convex.cloud",
            reference: "production",
            region: "aws-us-east-1",
          },
          {
            id: 13,
            name: "steady-hawk-789",
            deploymentType: "prod" as const,
            kind: "cloud" as const,
            isDefault: false,
            projectId: mockProject.id,
            creator: 1,
            createTime: Date.now() - 100000000,
            class: "s256",
            deploymentUrl: "https://steady-hawk-789.convex.cloud",
            reference: "prod/staging",
            region: "aws-eu-west-1",
          },
          // Ari's feature branch deployments
          {
            id: 21,
            name: "quick-panda-202",
            deploymentType: "dev" as const,
            kind: "cloud" as const,
            isDefault: false,
            projectId: mockProject.id,
            creator: 2,
            createTime: Date.now() - 72000000,
            class: "s256",
            deploymentUrl: "https://quick-panda-202.convex.cloud",
            reference: "dev/ari/auth-flow",
            region: "aws-us-east-1",
          },
          {
            id: 22,
            name: "calm-tiger-203",
            deploymentType: "dev" as const,
            kind: "cloud" as const,
            isDefault: false,
            projectId: mockProject.id,
            creator: 2,
            createTime: Date.now() - 60000000,
            class: "s256",
            deploymentUrl: "https://calm-tiger-203.convex.cloud",
            reference: "dev/ari/payment-v2",
            region: "aws-us-east-1",
          },
          {
            id: 23,
            name: "swift-eagle-204",
            deploymentType: "dev" as const,
            kind: "cloud" as const,
            isDefault: false,
            projectId: mockProject.id,
            creator: 2,
            createTime: Date.now() - 48000000,
            class: "s256",
            deploymentUrl: "https://swift-eagle-204.convex.cloud",
            reference: "dev/ari/onboarding",
            region: "aws-us-east-1",
          },
        ] satisfies PlatformDeploymentResponse[],
        isLoading: false,
      });
      return storyFn();
    },
  ],
  play: async ({ canvasElement }) => {
    const selectDeployment =
      await within(canvasElement).findByTestId("select-deployment");
    await userEvent.click(selectDeployment);
  },
};
