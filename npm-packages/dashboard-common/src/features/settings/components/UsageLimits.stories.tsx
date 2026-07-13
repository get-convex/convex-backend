import type { Meta, StoryObj } from "@storybook/nextjs";
import { useState, type ComponentProps } from "react";
import {
  UsageLimits,
  UsageLimit,
  UsageLimitConfig,
  computeUnbilledMetrics,
} from "./UsageLimits";

// Example limits spread across windows and both types so the segmented
// control's enabled/configured counts are non-trivial (Monthly 2/3, Daily 1/1,
// Hourly 1/1) and at least one metric card shows both rows filled.
const EXAMPLE_USAGE_LIMITS: UsageLimit[] = [
  {
    id: "example-1",
    metric: "functionCalls",
    limit: 10_000_000,
    window: "month",
    limitType: "warning",
    enabled: true,
  },
  {
    id: "example-2",
    metric: "functionCalls",
    limit: 50_000_000,
    window: "month",
    limitType: "disable",
    enabled: true,
  },
  {
    id: "example-3",
    metric: "databaseIoGb",
    limit: 500,
    window: "month",
    limitType: "warning",
    enabled: false,
  },
  {
    id: "example-4",
    metric: "actionComputeNodeJsGbHours",
    limit: 80,
    window: "day",
    limitType: "disable",
    enabled: true,
  },
  {
    id: "example-5",
    metric: "dataEgressGb",
    limit: 10,
    window: "hour",
    limitType: "warning",
    enabled: true,
  },
];

// A Business/Enterprise team on a non-dedicated deployment: Convex runtime and
// Query/Mutation compute aren't billed. Includes a configured limit for an
// unbilled metric in the Monthly window so it pins to the top with a callout.
const UNBILLED_METRICS = computeUnbilledMetrics({
  isBusinessPlan: true,
  isDedicated: false,
});
const UNBILLED_EXAMPLE: UsageLimit[] = [
  {
    id: "unbilled-1",
    metric: "actionComputeConvexGbHours",
    limit: 100,
    window: "month",
    limitType: "warning",
    enabled: true,
  },
  {
    id: "billed-1",
    metric: "functionCalls",
    limit: 50_000_000,
    window: "month",
    limitType: "disable",
    enabled: true,
  },
];

// Wrap in a stateful container so edits/additions/removals persist in the
// story. This mimics what the real API-backed manager does, but in memory.
// Every prop other than the CRUD callbacks is forwarded straight through.
function StatefulUsageLimits({
  usageLimits: initial,
  ...rest
}: ComponentProps<typeof UsageLimits>) {
  const [usageLimits, setUsageLimits] = useState<UsageLimit[]>(initial);
  let nextId = usageLimits.length;

  const onCreate = (config: UsageLimitConfig) => {
    nextId += 1;
    setUsageLimits((prev) => [...prev, { id: `example-${nextId}`, ...config }]);
  };
  const onUpdate = (id: string, config: UsageLimitConfig) => {
    setUsageLimits((prev) =>
      prev.map((limit) => (limit.id === id ? { id, ...config } : limit)),
    );
  };
  const onDelete = (id: string) => {
    setUsageLimits((prev) => prev.filter((limit) => limit.id !== id));
  };

  return (
    <UsageLimits
      {...rest}
      usageLimits={usageLimits}
      onCreate={onCreate}
      onUpdate={onUpdate}
      onDelete={onDelete}
    />
  );
}

// A typical standard-plan, non-dedicated deployment: CPU and Query/Mutation
// compute aren't billed, so those cards are hidden unless configured (rather
// than every metric showing at once). Stories can override this.
const STANDARD_UNBILLED = computeUnbilledMetrics({
  isBusinessPlan: false,
  isDedicated: false,
});

const meta = {
  component: UsageLimits,
  args: {
    onCreate: () => {},
    onUpdate: () => {},
    onDelete: () => {},
    unbilledMetrics: STANDARD_UNBILLED,
    deploymentType: "prod",
  },
  render: (args) => <StatefulUsageLimits {...args} />,
} satisfies Meta<typeof UsageLimits>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    usageLimits: EXAMPLE_USAGE_LIMITS,
  },
};

// No limits configured: every applicable metric still shows, with empty rows.
export const Empty: Story = {
  args: {
    usageLimits: [],
  },
};

export const Loading: Story = {
  args: {
    usageLimits: [],
    isLoading: true,
  },
};

// A member without write access: no Add/Edit buttons.
export const ReadOnly: Story = {
  args: {
    usageLimits: EXAMPLE_USAGE_LIMITS,
    canWrite: false,
  },
};

// Some metrics aren't billed on this plan/deployment. A configured unbilled
// metric pins to the top of its window with a callout.
export const WithUnbilledMetrics: Story = {
  args: {
    usageLimits: UNBILLED_EXAMPLE,
    unbilledMetrics: UNBILLED_METRICS,
  },
};

// A dev deployment: no email notifications are sent, so the warning threshold
// column is disabled with an explanation, and the disable threshold tooltip
// notes no email is sent.
export const DevDeployment: Story = {
  args: {
    usageLimits: EXAMPLE_USAGE_LIMITS,
    deploymentType: "dev",
  },
};

// Deployment type unknown (e.g. self-hosted): treated like prod/preview/custom,
// so both thresholds are available and tooltips mention emailing all team
// members.
export const UnknownDeploymentType: Story = {
  args: {
    usageLimits: EXAMPLE_USAGE_LIMITS,
    deploymentType: undefined,
  },
};
