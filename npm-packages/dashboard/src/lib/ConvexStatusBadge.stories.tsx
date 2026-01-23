import type { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexStatusBadge } from "./ConvexStatusBadge";

const meta: Meta<typeof ConvexStatusBadge> = {
  component: ConvexStatusBadge,
  parameters: {
    layout: "padded",
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const MinorIssues: Story = {
  args: {
    status: {
      indicator: "minor",
      description: "Minor Service Disruption",
    },
  },
};

export const MajorOutage: Story = {
  args: {
    status: {
      indicator: "major",
      description: "Major Service Outage",
    },
  },
};

export const CriticalOutage: Story = {
  args: {
    status: {
      indicator: "critical",
      description: "Critical System Failure",
    },
  },
};
