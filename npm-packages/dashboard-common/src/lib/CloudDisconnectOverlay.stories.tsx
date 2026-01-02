import type { Meta, StoryObj } from "@storybook/nextjs";
import { CloudDisconnectOverlay } from "./deploymentContext";
import { fn } from "storybook/test";

const meta: Meta<typeof CloudDisconnectOverlay> = {
  component: CloudDisconnectOverlay,
  parameters: {
    layout: "fullscreen",
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

const mockDeployment = {
  client: {} as any,
  httpClient: {} as any,
  deploymentUrl: "https://happy-animal-123.convex.cloud",
  adminKey: "mock-admin-key",
  deploymentName: "happy-animal-123",
};

const unreachableDeployment = {
  client: {} as any,
  httpClient: {} as any,
  deploymentUrl: "http://localhost:99999",
  adminKey: "mock-admin-key",
  deploymentName: "happy-animal-123",
};

export const CheckingHTTP: Story = {
  args: {
    deployment: mockDeployment,
    deploymentName: "happy-animal-123",
  },
};

export const HTTPReachable: Story = {
  args: {
    deployment: mockDeployment,
    deploymentName: "happy-animal-123",
  },
};

export const HTTPUnreachable: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
  },
};

export const LoadingStatus: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
    statusWidget: (
      <a
        href="https://status.convex.dev"
        target="_blank"
        rel="noreferrer"
        className="flex items-center gap-2 text-sm hover:underline"
      >
        <span className="text-content-secondary">Loading system status...</span>
      </a>
    ),
  },
};

export const AllOperational: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
    statusWidget: (
      <>
        <a
          href="https://status.convex.dev"
          target="_blank"
          rel="noreferrer"
          className="flex items-center gap-2 text-sm hover:underline"
        >
          <span className="relative flex size-3 shrink-0">
            <span className="relative inline-flex size-3 rounded-full bg-green-500" />
          </span>
          <span>All Systems Operational</span>
        </a>
        <p className="mt-2 text-xs text-content-secondary">
          For emerging issues, it may take the Convex team a few minutes to
          update system status.
        </p>
      </>
    ),
  },
};

export const MinorIssues: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
    statusWidget: (
      <a
        href="https://status.convex.dev"
        target="_blank"
        rel="noreferrer"
        className="flex items-center gap-2 text-sm hover:underline"
      >
        <span className="relative flex size-3 shrink-0">
          <span className="relative inline-flex size-3 rounded-full bg-yellow-500" />
        </span>
        <span>Minor Service Disruption</span>
      </a>
    ),
  },
};

export const MajorOutage: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
    statusWidget: (
      <a
        href="https://status.convex.dev"
        target="_blank"
        rel="noreferrer"
        className="flex items-center gap-2 text-sm hover:underline"
      >
        <span className="relative flex size-3 shrink-0">
          <span className="relative inline-flex size-3 rounded-full bg-orange-500" />
        </span>
        <span>Major Service Outage</span>
      </a>
    ),
  },
};

export const CriticalOutage: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
    statusWidget: (
      <a
        href="https://status.convex.dev"
        target="_blank"
        rel="noreferrer"
        className="flex items-center gap-2 text-sm hover:underline"
      >
        <span className="relative flex size-3 shrink-0">
          <span className="relative inline-flex size-3 rounded-full bg-red-500" />
        </span>
        <span>Critical System Failure</span>
      </a>
    ),
  },
};

export const WithSupportForm: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
    statusWidget: (
      <a
        href="https://status.convex.dev"
        target="_blank"
        rel="noreferrer"
        className="flex items-center gap-2 text-sm hover:underline"
      >
        <span className="relative flex size-3 shrink-0">
          <span className="relative inline-flex size-3 rounded-full bg-orange-500" />
        </span>
        <span>Major Service Outage</span>
      </a>
    ),
    openSupportForm: fn(),
  },
};

export const NoStatusInfo: Story = {
  args: {
    deployment: unreachableDeployment,
    deploymentName: "happy-animal-123",
  },
};

export const LocalDeployment: Story = {
  args: {
    deployment: {
      ...mockDeployment,
      deploymentName: "local-happy-animal",
    },
    deploymentName: "local-happy-animal",
  },
};
