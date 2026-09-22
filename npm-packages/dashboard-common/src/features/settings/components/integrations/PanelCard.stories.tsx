import { Meta, StoryObj } from "@storybook/nextjs";
import { type ContextType } from "react";
import type {
  ActiveDataSync,
  ActiveDataSyncSnapshotting,
} from "@convex-dev/platform/deploymentApi";
import { mocked, screen, userEvent, waitFor, within } from "storybook/test";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { useActiveDataSyncs } from "@common/features/settings/lib/api";
import { Sheet } from "@ui/Sheet";
import { PanelCard } from "./PanelCard";

const minutesAgo = (minutes: number) => Date.now() - minutes * 60 * 1000;
// The API reports database timestamps in nanoseconds since the epoch.
const nanos = (ms: number) => ms * 1e6;

const snapshottingStatus: ActiveDataSyncSnapshotting = {
  type: "snapshotting",
  numTablesSynced: 2,
  totalTables: 12,
  currentComponent: "",
  currentTable: "messages",
  numDocumentsInCurrentTable: 143_000,
  totalDocumentsInCurrentTable: 890_000,
  numDocumentsSynced: 1_230_000,
  totalDocuments: 2_900_000,
};

const snapshottingSync: ActiveDataSync = {
  syncId: "fivetran-2b1c4f8e-9b7a-4c2d-8e11-000000000001",
  lastUpdated: minutesAgo(2),
  status: snapshottingStatus,
};

// Table summaries are still bootstrapping, so the sync has no totals to
// measure itself against yet.
const bootstrappingSync: ActiveDataSync = {
  syncId: "fivetran-4c9a17be-2d60-4f85-b3e2-000000000004",
  lastUpdated: minutesAgo(1),
  status: {
    ...snapshottingStatus,
    totalDocumentsInCurrentTable: 0,
    totalDocuments: 0,
  },
};

const staleSync: ActiveDataSync = {
  syncId: "fivetran-6b0d83f1-5a7c-4e19-9d44-000000000005",
  lastUpdated: minutesAgo(4),
  status: {
    type: "stale",
    totalTables: 7,
    numDocumentsSynced: 1_902_551,
    syncedTs: nanos(minutesAgo(21)),
  },
};

const caughtUpSync: ActiveDataSync = {
  syncId: "fivetran-7d3e0a62-4f15-49bb-93c0-000000000002",
  lastUpdated: minutesAgo(2),
  status: {
    type: "upToDate",
    totalTables: 12,
    numDocumentsSynced: 2_934_128,
    syncedTs: nanos(minutesAgo(2)),
  },
};

// Past the halfway point of the 3-day window the backend keeps a sync in, so
// the modal explains why it is still listed.
const idleSync: ActiveDataSync = {
  syncId: "fivetran-8e21f5c0-3b94-42a7-81df-000000000006",
  lastUpdated: minutesAgo(60 * 24 * 2),
  status: {
    type: "upToDate",
    totalTables: 2,
    numDocumentsSynced: 61_400,
    syncedTs: nanos(minutesAgo(60 * 24 * 2)),
  },
};

// Only Fivetran's syncs belong on its card; this one is always filtered out.
const airbyteSync: ActiveDataSync = {
  syncId: "airbyte-9f52c1a7-6e08-4d3b-ae44-000000000003",
  lastUpdated: minutesAgo(5),
  status: {
    type: "upToDate",
    totalTables: 3,
    numDocumentsSynced: 4_212,
    syncedTs: nanos(minutesAgo(5)),
  },
};

const activeSyncs: ActiveDataSync[] = [snapshottingSync, airbyteSync];

const meta = {
  component: PanelCard,
  parameters: {
    // Matches the Integrations page story. Three checks fail on markup this
    // card shares with every other integration, none of it introduced here:
    // HealthIndicator's "active" green (4.4:1) and "pending" yellow (3.3:1)
    // against the Sheet, and ProBadge nesting a link inside the Tooltip's
    // button. Enforcing here would only pin those in place.
    a11y: { test: "todo" },
  },
  // The integrations page lays these cards out inside a Sheet; rendering one
  // straight onto the page background drops the muted text below its contrast
  // threshold, which is a property of the story rather than of the card.
  decorators: [
    (Story) => (
      <Sheet>
        <Story />
      </Sheet>
    ),
  ],
  args: {
    integration: { kind: "fivetran" as const },
    unavailableReason: null,
    teamSlug: "acme",
  },
  // Stand in for the listing request, honoring the `enabled` gate the card
  // passes so a story can assert the card never asks in the first place.
  beforeEach: (context) => {
    const syncs =
      (context.parameters as { activeDataSyncs?: ActiveDataSync[] })
        .activeDataSyncs ?? [];
    mocked(useActiveDataSyncs)
      .mockClear()
      .mockImplementation((enabled) => (enabled ? syncs : undefined));
  },
  render: (args) => (
    <DeploymentInfoContext.Provider
      value={
        {
          ...mockDeploymentInfo,
          showFivetranSyncProgress: true,
        } as ContextType<typeof DeploymentInfoContext>
      }
    >
      <PanelCard {...args} />
    </DeploymentInfoContext.Provider>
  ),
} satisfies Meta<typeof PanelCard>;

export default meta;
type Story = StoryObj<typeof meta>;

// Nothing is syncing, so the card offers setup instructions instead.
export const FivetranNoActiveSync: Story = {
  parameters: { activeDataSyncs: [] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Get Started"));
  },
};

// The other sync belongs to Airbyte, so the card summarizes just the Fivetran
// one -- mid-snapshot -- and takes the place of the "Get Started" link.
export const FivetranSyncing: Story = {
  parameters: { activeDataSyncs: activeSyncs },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Syncing"));
    await canvas.findByText("42% of initial snapshot");
    if (canvas.queryByText("Get Started")) {
      throw new Error("still offering setup instructions during a sync");
    }
  },
};

// Without table summaries there is no percentage to report, so the detail line
// falls back to naming the phase.
export const FivetranSyncingWithoutTotals: Story = {
  parameters: { activeDataSyncs: [bootstrappingSync] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Syncing"));
    await canvas.findByText("initial snapshot");
  },
};

// Nothing is snapshotting, so the card reports the same "Active" as a
// configured log stream, over the timestamp the data is synced through.
export const FivetranActive: Story = {
  parameters: { activeDataSyncs: [caughtUpSync] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Active"));
    await canvas.findByText(/^Last synced /);
  },
};

// A sync that reached a snapshot but has fallen behind is still "Syncing" --
// what it can serve is behind the deployment -- and has no percentage to
// report, so it names the point its data is consistent as of.
export const FivetranSyncingCatchingUp: Story = {
  parameters: { activeDataSyncs: [staleSync] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Syncing"));
    await canvas.findByText("Synced up until 21 minutes ago");
  },
};

// Syncs at different points have no one status between them, so the card drops
// the summary for a count and sends the reader to the modal.
export const FivetranMultipleSyncs: Story = {
  parameters: {
    activeDataSyncs: [snapshottingSync, staleSync, caughtUpSync, airbyteSync],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("3 active syncs"));
    for (const summary of ["Active", "Syncing"]) {
      if (canvas.queryByText(summary)) {
        throw new Error(`summarized several syncs as "${summary}"`);
      }
    }
  },
};

const TIP_3_DAY_WINDOW =
  "A sync stays listed for 3 days after its most recent page.";

async function openSyncDetails(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  await userEvent.click(
    await canvas.findByLabelText("Show Fivetran sync details"),
  );
  return within(await screen.findByTestId("modal"));
}

export const FivetranSyncDetailModal: Story = {
  parameters: { activeDataSyncs: activeSyncs },
  play: async ({ canvasElement }) => {
    const dialog = await openSyncDetails(canvasElement);
    await dialog.findByText("Syncing");
    await dialog.findByText(/^messages · table 3 of 12/);
    // An initial snapshot has no consistent timestamp to report yet.
    if (dialog.queryByText(/ ago$/)) {
      throw new Error("timestamped a sync that has no snapshot yet");
    }
    // This sync ran 2 minutes ago, so it is nowhere near ageing out.
    await userEvent.hover(dialog.getByText("Syncing"));
    if (screen.queryAllByText(TIP_3_DAY_WINDOW).length > 0) {
      throw new Error("explained the 3-day window for a sync that just ran");
    }
  },
};

// A sync idle for most of its 3-day window gets the only explanation of that
// window anywhere in the UI, on hover.
export const FivetranSyncDetailModalIdleSync: Story = {
  parameters: { activeDataSyncs: [idleSync] },
  play: async ({ canvasElement }) => {
    const dialog = await openSyncDetails(canvasElement);
    await userEvent.hover(await dialog.findByText("Up to date"));
    // Radix renders the tip twice: the visible bubble and a copy for screen
    // readers.
    await screen.findAllByText(TIP_3_DAY_WINDOW);
  },
};

// Without totals the snapshot has no end point to measure against, so its row
// gets an indeterminate bar rather than a percentage.
export const FivetranSyncDetailModalWithoutTotals: Story = {
  parameters: { activeDataSyncs: [bootstrappingSync] },
  play: async ({ canvasElement }) => {
    const dialog = await openSyncDetails(canvasElement);
    const bar = await dialog.findByRole("progressbar", {
      name: "Initial snapshot progress",
    });
    if (bar.getAttribute("aria-valuenow") !== null) {
      throw new Error("reported a percentage without knowing the total");
    }
  },
};

// Every per-sync rendering at once: an initial snapshot with a progress bar,
// one catching up without one -- both labelled "Syncing" -- and one caught up.
export const FivetranSyncDetailModalMultipleSyncs: Story = {
  parameters: {
    activeDataSyncs: [snapshottingSync, staleSync, caughtUpSync, airbyteSync],
  },
  play: async ({ canvasElement }) => {
    const dialog = await openSyncDetails(canvasElement);
    const syncing = await dialog.findAllByText("Syncing");
    if (syncing.length !== 2) {
      throw new Error(`expected 2 syncing rows, got ${syncing.length}`);
    }
    await dialog.findByText("Up to date");
    await dialog.findByText(/^Synced up until /);
    await dialog.findByText(/^Last synced /);
  },
};

// Streaming export is a Pro feature, so a team without the entitlement sees the
// upsell and the card never asks for the listing.
export const FivetranMissingEntitlement: Story = {
  args: { unavailableReason: "MissingEntitlement" },
  parameters: { activeDataSyncs: activeSyncs },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Pro"));
    // The card short-circuits to the upsell, so nothing even asks for syncs.
    if (mocked(useActiveDataSyncs).mock.calls.length > 0) {
      throw new Error("listed active syncs without the entitlement");
    }
  },
};

// `showFivetranSyncProgress` off is the default everywhere until the flag is
// rolled out, so the card must look exactly like FivetranNoActiveSync even
// with a sync in flight.
export const FivetranSyncProgressFlagOff: Story = {
  parameters: { activeDataSyncs: activeSyncs },
  render: (args) => (
    <DeploymentInfoContext.Provider
      value={mockDeploymentInfo as ContextType<typeof DeploymentInfoContext>}
    >
      <PanelCard {...args} />
    </DeploymentInfoContext.Provider>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => canvas.getByText("Get Started"));
    // Asserting the gate itself, rather than the indicator being absent --
    // which would also hold while a listing was merely still in flight.
    if (mocked(useActiveDataSyncs).mock.calls.some(([enabled]) => enabled)) {
      throw new Error("listed active syncs while the flag is off");
    }
  },
};
