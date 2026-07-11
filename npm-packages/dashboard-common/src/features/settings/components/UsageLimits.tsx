import { useCallback, useEffect, useState, type ReactNode } from "react";
import { useRouter } from "next/router";
import {
  PlusCircledIcon,
  DotsVerticalIcon,
  ExclamationTriangleIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { Menu, MenuItem } from "@ui/Menu";
import { Checkbox } from "@ui/Checkbox";
import { TextInput } from "@ui/TextInput";
import { Tooltip } from "@ui/Tooltip";
import { Loading } from "@ui/Loading";
import { SegmentedControl } from "@ui/SegmentedControl";
import { cn } from "@ui/cn";

// The usage metrics that a deployment-level usage limit can be applied to.
// These match the `UsageLimitMetric` enum on the backend (serialized with
// serde/strum `camelCase`), so the string values are sent to the usage limits
// API verbatim.
export type UsageMetric =
  | "functionCalls"
  | "queryMutationComputeGbHours"
  | "actionComputeConvexGbHours"
  | "actionComputeNodeJsGbHours"
  | "actionComputeCpuGbHours"
  | "databaseIoGb"
  | "searchQueryGb"
  | "dataEgressGb";

// The window over which a usage limit is enforced. Matches the backend
// `UsageLimitWindow` enum.
export type UsageLimitWindow = "hour" | "day" | "month";

// What happens when the limit is exceeded. Matches the backend
// `UsageLimitType` enum.
// - "warning": email all team members.
// - "disable": email all team members and disable the deployment for the rest
//   of the usage limit window.
export type UsageLimitType = "warning" | "disable";

// The editable configuration of a usage limit. Mirrors the backend
// `UsageLimitConfigRequest` shape so it can be sent to the API as-is.
export type UsageLimitConfig = {
  // Optional human-readable label. Not currently editable in the UI.
  name?: string | null;
  metric: UsageMetric;
  // The limit, as a count of the metric's `rawUnit`s.
  limit: number;
  window: UsageLimitWindow;
  limitType: UsageLimitType;
  enabled: boolean;
};

// A saved usage limit: its configuration plus the id assigned by the backend.
// Mirrors the backend `UsageLimitConfigResponse` shape.
export type UsageLimit = UsageLimitConfig & {
  id: string;
};

// The largest limit value the UI accepts: 100 trillion of the metric's raw
// unit. Guards against fat-fingered values the backend would otherwise store.
export const MAX_USAGE_LIMIT_VALUE = 100_000_000_000_000;

type MetricConfig = {
  name: string;
  description: string;
  // Long unit label used in raw mode (e.g. "GB-hours").
  rawUnit: string;
  // Compact unit label shown inline next to inputs (e.g. "GBh").
  rawUnitShort: string;
  // Increment used by the numeric input, and the placeholder amount hint.
  rawStep: number;
};

export const METRIC_CONFIG: Record<UsageMetric, MetricConfig> = {
  functionCalls: {
    name: "Function calls",
    description:
      "Total number of query, mutation, action, HTTP action, and file storage calls.",
    rawUnit: "function calls",
    rawUnitShort: "calls",
    rawStep: 1_000_000,
  },
  queryMutationComputeGbHours: {
    name: "Query/Mutation compute",
    description: "Compute consumed running queries and mutations.",
    rawUnit: "GB-hours",
    rawUnitShort: "GBh",
    rawStep: 1,
  },
  actionComputeConvexGbHours: {
    name: "Action compute (Convex runtime)",
    description: "Compute consumed running actions in the Convex runtime.",
    rawUnit: "GB-hours",
    rawUnitShort: "GBh",
    rawStep: 1,
  },
  actionComputeNodeJsGbHours: {
    name: "Action compute (Node.js)",
    description: "Compute consumed running actions in the Node.js runtime.",
    rawUnit: "GB-hours",
    rawUnitShort: "GBh",
    rawStep: 1,
  },
  actionComputeCpuGbHours: {
    name: "Action compute (CPU)",
    description: "CPU time consumed running actions.",
    rawUnit: "GB-hours",
    rawUnitShort: "GBh",
    rawStep: 1,
  },
  databaseIoGb: {
    name: "Database I/O",
    description: "Bandwidth used reading from and writing to the database.",
    rawUnit: "GB",
    rawUnitShort: "GB",
    rawStep: 1,
  },
  searchQueryGb: {
    name: "Search queries",
    description: "Bandwidth used serving text and vector search queries.",
    rawUnit: "query-GB",
    rawUnitShort: "qGB",
    rawStep: 1,
  },
  dataEgressGb: {
    name: "Data egress",
    description:
      "Bandwidth used serving file downloads, outgoing fetch requests, and log stream egress.",
    rawUnit: "GB",
    rawUnitShort: "GB",
    rawStep: 1,
  },
};

const METRIC_ORDER: UsageMetric[] = [
  "functionCalls",
  "queryMutationComputeGbHours",
  "actionComputeConvexGbHours",
  "actionComputeNodeJsGbHours",
  "actionComputeCpuGbHours",
  "databaseIoGb",
  "searchQueryGb",
  "dataEgressGb",
];

// What each limit type does when the limit is exceeded, shown as a tooltip on
// the row's type label.
const ACTION_DESCRIPTION: Record<UsageLimitType, string> = {
  warning: "When exceeded, Convex emails all team members.",
  disable:
    "When exceeded, Convex emails all team members and disables the deployment for the rest of the window.",
};

// Short label for each limit type. A metric card shows one row per type.
export const LIMIT_TYPE_LABEL: Record<UsageLimitType, string> = {
  warning: "Warning threshold",
  disable: "Disable threshold",
};

const LIMIT_TYPE_ORDER: UsageLimitType[] = ["warning", "disable"];

// The window segmented control, ordered coarsest-first per the design.
const WINDOW_ORDER: UsageLimitWindow[] = ["month", "day", "hour"];
const WINDOW_LABEL: Record<UsageLimitWindow, string> = {
  month: "Monthly",
  day: "Daily",
  hour: "Hourly",
};

// Suffix shown after a limit's unit to convey the window it's enforced over
// (e.g. "10 GB / month").
export const WINDOW_SUFFIX: Record<UsageLimitWindow, string> = {
  month: "/ month",
  day: "/ day",
  hour: "/ hour",
};

// Explains when usage resets for the selected window, shown beneath the window
// segmented control.
const WINDOW_RESET_DESCRIPTION: Record<UsageLimitWindow, string> = {
  month: "Monthly usage resets on the first of the month, at midnight UTC.",
  day: "Daily usage resets at midnight UTC.",
  hour: "Hourly usage resets at the beginning of each hour.",
};

export const AMOUNT_FORMAT = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
});

// Shared width for each threshold column so the Warning and Disable columns
// line up, and so a column's read-only view and inline editor are the same
// width (switching between them causes no layout shift).
const THRESHOLD_COL = "w-64";

// Which compute metrics a team is billed for depends on plan tier and
// deployment class (see convex.dev/pricing and convex.dev/enterprise/pricing):
// - Node.js action compute is billed on every plan.
// - Convex-runtime action compute is billed only on non-Business/Enterprise
//   plans.
// - CPU action compute is billed only on Business/Enterprise plans.
// - Query/Mutation compute is billed only on dedicated (DXXXX) deployments.
// Returns a map from each metric the team ISN'T billed for to a short
// explanation; billed metrics are absent from the map. A limit on an unbilled
// metric is still enforced when enabled; the team just isn't charged for that
// metric's usage.
export function computeUnbilledMetrics({
  isBusinessPlan,
  isDedicated,
}: {
  isBusinessPlan: boolean;
  isDedicated: boolean;
}): Partial<Record<UsageMetric, string>> {
  const unbilled: Partial<Record<UsageMetric, string>> = {};
  if (isBusinessPlan) {
    unbilled.actionComputeConvexGbHours =
      "Your plan isn't billed for Convex runtime compute (Business and Enterprise plans are billed for CPU time instead), but this limit is still enforced when enabled.";
  } else {
    unbilled.actionComputeCpuGbHours =
      "Your plan isn't billed for CPU time (only Business and Enterprise plans are), but this limit is still enforced when enabled.";
  }
  if (!isDedicated) {
    unbilled.queryMutationComputeGbHours =
      "Your deployment isn't billed for Query/Mutation compute (only dedicated deployments are), but this limit is still enforced when enabled.";
  }
  return unbilled;
}

// A callout shown when a metric isn't billed on the current plan/deployment.
// content-warning on background-warning clears the 4.5:1 contrast bar (≈ 5.8:1).
function UnbilledMetricNote({ reason }: { reason: string }) {
  return (
    <div className="flex w-fit items-start gap-2 rounded-md bg-background-warning p-2 text-xs text-content-warning">
      <ExclamationTriangleIcon className="mt-0.5 shrink-0" />
      <span className="max-w-prose">{reason}</span>
    </div>
  );
}

// Warn the user before they navigate away — via browser unload (close/refresh)
// or a client-side route change — while any inline editor has unsaved edits.
function useUnsavedChangesWarning(when: boolean) {
  const router = useRouter();
  useEffect(() => {
    if (!when) {
      return undefined;
    }
    const message =
      "You have unsaved usage limit changes. Leave without saving?";
    const beforeUnload = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = message;
      return message;
    };
    const onRouteChangeStart = (url: string) => {
      if (url === router.asPath) {
        return;
      }

      if (!window.confirm(message)) {
        router.events.emit("routeChangeError");
        // Throwing aborts the in-flight route change (Next.js pages router).

        throw `Route change to "${url}" aborted (unsaved usage limit changes).`;
      }
    };
    window.addEventListener("beforeunload", beforeUnload);
    router.events.on("routeChangeStart", onRouteChangeStart);
    return () => {
      window.removeEventListener("beforeunload", beforeUnload);
      router.events.off("routeChangeStart", onRouteChangeStart);
    };
  }, [when, router]);
}

export function UsageLimits({
  usageLimits,
  onCreate,
  onUpdate,
  onDelete,
  canWrite = true,
  isLoading = false,
  title = "Usage Limits",
  unbilledMetrics = {},
  writePermissionTip = "You do not have permission to modify usage limits.",
}: {
  usageLimits: UsageLimit[];
  // Persist a brand new usage limit.
  onCreate: (config: UsageLimitConfig) => Promise<void> | void;
  // Persist the full configuration of an existing usage limit.
  onUpdate: (id: string, config: UsageLimitConfig) => Promise<void> | void;
  // Delete an existing usage limit.
  onDelete: (id: string) => Promise<void> | void;
  // Whether the current member may modify usage limits.
  canWrite?: boolean;
  // Whether the usage limits are still loading.
  isLoading?: boolean;
  // Heading for the card. Defaults to the deployment-level title.
  title?: string;
  // Metrics that aren't billed on the current plan/deployment, mapped to a
  // short explanation. See `computeUnbilledMetrics`. Absent = billed.
  unbilledMetrics?: Partial<Record<UsageMetric, string>>;
  // Tooltip shown on disabled write controls when `canWrite` is false. Callers
  // pass a `PermissionDeniedTip` so custom-role members see the missing action.
  writePermissionTip?: ReactNode;
}) {
  const [selectedWindow, setSelectedWindow] =
    useState<UsageLimitWindow>("month");
  // Keys (`${metric}|${window}|${limitType}`) of the threshold columns whose
  // inline editor is currently open. Several may be open at once.
  const [editingKeys, setEditingKeys] = useState<Set<string>>(new Set());
  // The subset of open editors that have unsaved changes.
  const [dirtyKeys, setDirtyKeys] = useState<Set<string>>(new Set());

  useUnsavedChangesWarning(dirtyKeys.size > 0);

  const startEdit = useCallback((key: string) => {
    setEditingKeys((prev) => {
      const next = new Set(prev);
      next.add(key);
      return next;
    });
  }, []);
  const stopEdit = useCallback((key: string) => {
    setEditingKeys((prev) => {
      const next = new Set(prev);
      next.delete(key);
      return next;
    });
  }, []);
  const setDirty = useCallback((key: string, dirty: boolean) => {
    setDirtyKeys((prev) => {
      if (dirty === prev.has(key)) {
        return prev;
      }
      const next = new Set(prev);
      if (dirty) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  }, []);

  // Each window segment is labelled with a badge showing how many of its limits
  // are enabled out of how many are configured (e.g. "Monthly 3/4 Enabled"); the
  // badge is hidden when nothing is configured for that window.
  const windowOptions = WINDOW_ORDER.map((w) => {
    const inWindow = usageLimits.filter((limit) => limit.window === w);
    const enabled = inWindow.filter((limit) => limit.enabled).length;
    return {
      value: w,
      label: (
        <span className="flex items-center gap-1.5">
          {WINDOW_LABEL[w]}
          {inWindow.length > 0 && (
            <span className="rounded-full bg-blue-100 px-1.5 py-0.5 text-xs font-medium text-blue-700 dark:bg-util-accent dark:text-white">
              <span className="tabular-nums">
                {enabled}/{inWindow.length}
              </span>{" "}
              Enabled
            </span>
          )}
        </span>
      ),
    };
  });

  const limitFor = (metric: UsageMetric, limitType: UsageLimitType) =>
    usageLimits.find(
      (limit) =>
        limit.metric === metric &&
        limit.window === selectedWindow &&
        limit.limitType === limitType,
    );

  // Unbilled metrics that have a limit configured in the selected window are
  // pinned to the top so they're noticeable; applicable metrics always follow.
  const unbilledConfigured = METRIC_ORDER.filter(
    (m) =>
      unbilledMetrics[m] && (limitFor(m, "warning") || limitFor(m, "disable")),
  );
  const applicable = METRIC_ORDER.filter((m) => !unbilledMetrics[m]);
  const shownMetrics = [...unbilledConfigured, ...applicable];

  return (
    <Sheet className="mb-6 flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <h3>{title}</h3>
        <p className="max-w-prose text-sm text-content-secondary">
          Limit how much usage this deployment can consume in a given timeframe.
        </p>
      </div>

      <div className="flex flex-col gap-2">
        <Tooltip
          asChild
          tip="Configure limits at a monthly, daily, or hourly granularity. Each window's usage is tracked and enforced separately."
          side="right"
        >
          <span className="inline-flex w-fit cursor-help items-center gap-1 text-sm text-content-secondary">
            Window
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </span>
        </Tooltip>

        <SegmentedControl
          className="w-fit"
          options={windowOptions}
          value={selectedWindow}
          onChange={(w) => {
            // Switching windows unmounts the current window's editors, so warn
            // before discarding any unsaved edits (mirrors the nav guard).
            if (
              dirtyKeys.size > 0 &&
              !window.confirm(
                "You have unsaved usage limit changes. Switch window without saving?",
              )
            ) {
              return;
            }
            setSelectedWindow(w);
            setEditingKeys(new Set());
          }}
        />

        <p className="text-xs text-content-secondary">
          {WINDOW_RESET_DESCRIPTION[selectedWindow]}
        </p>
      </div>

      {isLoading ? (
        <Loading fullHeight={false} className="h-24 w-full rounded-lg" />
      ) : (
        <div className="flex flex-col divide-y divide-border-transparent">
          {shownMetrics.map((metric) => (
            <UsageLimitMetricCard
              key={metric}
              metric={metric}
              window={selectedWindow}
              warningLimit={limitFor(metric, "warning")}
              disableLimit={limitFor(metric, "disable")}
              unbilledReason={unbilledMetrics[metric]}
              canWrite={canWrite}
              writePermissionTip={writePermissionTip}
              editingKeys={editingKeys}
              onStartEdit={startEdit}
              onStopEdit={stopEdit}
              onDirtyChange={setDirty}
              onCreate={onCreate}
              onUpdate={onUpdate}
              onDelete={onDelete}
            />
          ))}
        </div>
      )}
    </Sheet>
  );
}

// One card per metric: header + a Warning column and a Disable column, shown
// side by side for the selected window.
function UsageLimitMetricCard({
  metric,
  window,
  warningLimit,
  disableLimit,
  unbilledReason,
  canWrite,
  writePermissionTip,
  editingKeys,
  onStartEdit,
  onStopEdit,
  onDirtyChange,
  onCreate,
  onUpdate,
  onDelete,
}: {
  metric: UsageMetric;
  window: UsageLimitWindow;
  warningLimit?: UsageLimit;
  disableLimit?: UsageLimit;
  unbilledReason?: string;
  canWrite: boolean;
  writePermissionTip: ReactNode;
  editingKeys: Set<string>;
  onStartEdit: (key: string) => void;
  onStopEdit: (key: string) => void;
  onDirtyChange: (key: string, dirty: boolean) => void;
  onCreate: (config: UsageLimitConfig) => Promise<void> | void;
  onUpdate: (id: string, config: UsageLimitConfig) => Promise<void> | void;
  onDelete: (id: string) => Promise<void> | void;
}) {
  const config = METRIC_CONFIG[metric];
  const limitByType: Record<UsageLimitType, UsageLimit | undefined> = {
    warning: warningLimit,
    disable: disableLimit,
  };
  return (
    <div className="flex flex-col gap-3 py-4">
      <div className="flex flex-col">
        <span className="font-medium text-content-primary">{config.name}</span>
        <span className="text-xs text-content-secondary">
          {config.description}
        </span>
      </div>

      {unbilledReason && <UnbilledMetricNote reason={unbilledReason} />}

      <div className="flex flex-wrap gap-8">
        {LIMIT_TYPE_ORDER.map((limitType) => {
          const rowKey = `${metric}|${window}|${limitType}`;
          return (
            <UsageLimitThreshold
              key={limitType}
              metric={metric}
              window={window}
              limitType={limitType}
              limit={limitByType[limitType]}
              canWrite={canWrite}
              writePermissionTip={writePermissionTip}
              isEditing={editingKeys.has(rowKey)}
              onStartEdit={() => onStartEdit(rowKey)}
              onStopEdit={() => onStopEdit(rowKey)}
              onDirtyChange={onDirtyChange}
              onCreate={onCreate}
              onUpdate={onUpdate}
              onDelete={onDelete}
            />
          );
        })}
      </div>
    </div>
  );
}

// A threshold column's label ("Warning threshold"/"Disable threshold") with a
// tooltip describing what happens when the limit is exceeded.
function ThresholdLabel({ limitType }: { limitType: UsageLimitType }) {
  return (
    <Tooltip tip={ACTION_DESCRIPTION[limitType]} side="right">
      <span className="inline-flex cursor-help items-center gap-1 text-sm text-content-secondary">
        {LIMIT_TYPE_LABEL[limitType]}
        <QuestionMarkCircledIcon className="text-content-tertiary" />
      </span>
    </Tooltip>
  );
}

// A single (metric, window, limitType) threshold column. Read-only when not
// editing (label on top, then the configured amount + status, with an overflow
// menu for edit/delete), swapping to an inline editor while editing.
function UsageLimitThreshold({
  metric,
  window,
  limitType,
  limit,
  canWrite,
  writePermissionTip,
  isEditing,
  onStartEdit,
  onStopEdit,
  onDirtyChange,
  onCreate,
  onUpdate,
  onDelete,
}: {
  metric: UsageMetric;
  window: UsageLimitWindow;
  limitType: UsageLimitType;
  limit?: UsageLimit;
  canWrite: boolean;
  writePermissionTip: ReactNode;
  isEditing: boolean;
  onStartEdit: () => void;
  onStopEdit: () => void;
  onDirtyChange: (key: string, dirty: boolean) => void;
  onCreate: (config: UsageLimitConfig) => Promise<void> | void;
  onUpdate: (id: string, config: UsageLimitConfig) => Promise<void> | void;
  onDelete: (id: string) => Promise<void> | void;
}) {
  const config = METRIC_CONFIG[metric];

  if (isEditing) {
    return (
      <UsageLimitThresholdEditor
        metric={metric}
        window={window}
        limitType={limitType}
        limit={limit}
        onDone={onStopEdit}
        onDirtyChange={onDirtyChange}
        onCreate={onCreate}
        onUpdate={onUpdate}
      />
    );
  }

  return (
    <div className={cn(THRESHOLD_COL, "flex flex-col gap-1")}>
      <div className="flex min-h-6 items-center gap-4">
        <ThresholdLabel limitType={limitType} />
        {limit && (
          <ThresholdOverflowMenu
            canWrite={canWrite}
            writePermissionTip={writePermissionTip}
            onEdit={onStartEdit}
            onDelete={() => onDelete(limit.id)}
          />
        )}
      </div>
      {limit ? (
        <>
          <div className="flex items-baseline gap-1">
            <span className="text-base text-content-primary">
              {AMOUNT_FORMAT.format(limit.limit)} {config.rawUnitShort}
            </span>
            <span className="text-sm text-content-secondary">
              {WINDOW_SUFFIX[window]}
            </span>
          </div>
          <StatusBadge enabled={limit.enabled} />
        </>
      ) : (
        <ConfigureLimitButton
          canWrite={canWrite}
          writePermissionTip={writePermissionTip}
          onStartEdit={onStartEdit}
        />
      )}
    </div>
  );
}

// The "Configure limit" button shown in an empty slot. When the member can't
// write it's disabled with an explanatory tooltip rather than hidden.
function ConfigureLimitButton({
  canWrite,
  writePermissionTip,
  onStartEdit,
}: {
  canWrite: boolean;
  writePermissionTip: ReactNode;
  onStartEdit: () => void;
}) {
  return (
    <Button
      className="w-fit"
      size="xs"
      variant="neutral"
      inline
      icon={<PlusCircledIcon />}
      tip={!canWrite ? writePermissionTip : undefined}
      disabled={!canWrite}
      onClick={onStartEdit}
    >
      Configure limit
    </Button>
  );
}

// The overflow menu for a configured threshold: Edit opens the inline editor,
// Delete removes the limit. Both are disabled with a tooltip when the member
// can't write.
function ThresholdOverflowMenu({
  canWrite,
  writePermissionTip,
  onEdit,
  onDelete,
}: {
  canWrite: boolean;
  writePermissionTip: ReactNode;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const disabledTip = !canWrite ? writePermissionTip : undefined;
  return (
    <Menu
      placement="bottom-end"
      buttonProps={{
        "aria-label": "Limit options",
        icon: <DotsVerticalIcon />,
        size: "xs",
        variant: "neutral",
      }}
    >
      <MenuItem
        action={onEdit}
        disabled={!canWrite}
        tip={disabledTip}
        tipSide="left"
      >
        Edit
      </MenuItem>
      <MenuItem
        action={onDelete}
        variant="danger"
        disabled={!canWrite}
        tip={disabledTip}
        tipSide="left"
      >
        Delete
      </MenuItem>
    </Menu>
  );
}

// Inline editor for a single threshold column. Mounted fresh when a column
// enters edit mode, so its local state is seeded from the current limit (or
// blank for a new one).
function UsageLimitThresholdEditor({
  metric,
  window,
  limitType,
  limit,
  onDone,
  onDirtyChange,
  onCreate,
  onUpdate,
}: {
  metric: UsageMetric;
  window: UsageLimitWindow;
  limitType: UsageLimitType;
  limit?: UsageLimit;
  onDone: () => void;
  onDirtyChange: (key: string, dirty: boolean) => void;
  onCreate: (config: UsageLimitConfig) => Promise<void> | void;
  onUpdate: (id: string, config: UsageLimitConfig) => Promise<void> | void;
}) {
  const config = METRIC_CONFIG[metric];
  // New limits start with a blank amount for the user to fill in, and default
  // to enforced.
  const [amount, setAmount] = useState(limit ? String(limit.limit) : "");
  const [enabled, setEnabled] = useState(limit ? limit.enabled : true);
  const [isSaving, setIsSaving] = useState(false);

  const parsedLimit = Math.floor(Number(amount));
  const hasAmount = amount.trim() !== "";
  // The UI enforces 1 <= limit <= 100 trillion.
  const isInRange =
    Number.isFinite(parsedLimit) &&
    parsedLimit >= 1 &&
    parsedLimit <= MAX_USAGE_LIMIT_VALUE;
  const isValid = hasAmount && isInRange;

  // Report whether this editor has unsaved changes so the page can warn before
  // navigating away. An existing limit is dirty when a field diverges from the
  // saved value; a new one is dirty once the user touches it.
  const rowKey = `${metric}|${window}|${limitType}`;
  const isDirty = limit
    ? amount !== String(limit.limit) || enabled !== limit.enabled
    : hasAmount || enabled !== true;
  useEffect(() => {
    onDirtyChange(rowKey, isDirty);
    return () => onDirtyChange(rowKey, false);
  }, [rowKey, isDirty, onDirtyChange]);

  const handleSave = async () => {
    if (!isValid) {
      return;
    }
    setIsSaving(true);
    try {
      if (limit) {
        await onUpdate(limit.id, {
          name: limit.name,
          metric,
          limit: parsedLimit,
          window,
          limitType,
          enabled,
        });
      } else {
        await onCreate({
          metric,
          limit: parsedLimit,
          window,
          limitType,
          enabled,
        });
      }
      // Only close the editor once the save actually succeeded. onCreate/onUpdate
      // reject on API failure (after toasting), so a failed save keeps the editor
      // open and the in-progress edits intact.
      onDone();
    } catch {
      // The mutation hook already surfaced the error via toast; swallow it here
      // so the editor stays open rather than dropping the user's edits.
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className={cn(THRESHOLD_COL, "flex flex-col gap-2")}>
      <div className="flex min-h-6 items-center">
        <ThresholdLabel limitType={limitType} />
      </div>
      <div className="flex flex-col gap-1">
        <TextInput
          id={`usage-limit-${metric}-${limitType}`}
          label="Amount"
          labelHidden
          type="number"
          min={1}
          max={MAX_USAGE_LIMIT_VALUE}
          step={config.rawStep}
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          rightAddon={
            <span className="text-xs text-content-secondary">
              {config.rawUnitShort} {WINDOW_SUFFIX[window]}
            </span>
          }
        />
        {hasAmount && !isInRange && (
          <p className="text-xs text-content-errorSecondary">
            Enter an amount between 1 and 100 trillion.
          </p>
        )}
      </div>
      <Tooltip
        tip="When enabled, this limit is enforced: if usage exceeds the allotted amount, the limit takes effect."
        side="bottom"
      >
        <label className="flex w-fit cursor-help items-center gap-2 text-xs text-content-secondary">
          <Checkbox
            checked={enabled}
            onChange={() => setEnabled((prev) => !prev)}
          />
          Enabled
        </label>
      </Tooltip>
      <div className="flex items-center gap-2">
        <Button
          size="xs"
          inline
          onClick={handleSave}
          loading={isSaving}
          disabled={!isValid}
        >
          Save
        </Button>
        <Button
          size="xs"
          variant="neutral"
          inline
          onClick={onDone}
          disabled={isSaving}
        >
          Cancel
        </Button>
      </div>
    </div>
  );
}

function StatusBadge({ enabled }: { enabled: boolean }) {
  return (
    <Tooltip
      asChild
      tip={
        enabled
          ? "This limit is enabled: if usage exceeds the allotted amount, the limit will be enforced."
          : "This limit is disabled: it will not be enforced even if usage exceeds the allotted amount."
      }
      side="bottom"
    >
      <span
        className={cn(
          "w-fit cursor-help rounded-full px-2 py-0.5 text-xs font-medium text-content-primary",
          enabled ? "bg-background-success" : "bg-background-tertiary",
        )}
      >
        {enabled ? "Enabled" : "Disabled"}
      </span>
    </Tooltip>
  );
}
