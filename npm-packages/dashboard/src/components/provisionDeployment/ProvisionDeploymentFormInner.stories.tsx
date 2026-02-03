import type { Meta, StoryObj } from "@storybook/nextjs";
import { fn } from "storybook/test";
import { ProvisionDeploymentFormInner } from "./ProvisionDeploymentForm";

const meta = {
  component: ProvisionDeploymentFormInner,
  args: {
    deploymentType: "prod",
    regions: [
      {
        displayName: "Europe (Ireland)",
        name: "aws-eu-west-1",
        available: true,
      },
      {
        displayName: "US East (N. Virginia)",
        name: "aws-us-east-1",
        available: true,
      },
    ],
    onCreate: fn(),
    teamSlug: "example-team",
    teamName: "Example Team",
    isAdmin: true,
  },
} satisfies Meta<typeof ProvisionDeploymentFormInner>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const EuropeNotAvailable: Story = {
  args: {
    regions: [
      {
        displayName: "Europe (Ireland)",
        name: "aws-eu-west-1",
        available: false,
      },
      {
        displayName: "US East (N. Virginia)",
        name: "aws-us-east-1",
        available: true,
      },
    ],
    teamSlug: "example-team",
  },
};

export const Loading: Story = {
  args: {
    regions: undefined,
  },
};

export const Development: Story = {
  args: {
    deploymentType: "dev",
    teamSlug: "example-team",
  },
};

export const Local: Story = {
  args: {
    regions: [
      {
        displayName: "test",
        name: "local",
        available: true,
      },
    ],
  },
};

export const NotAdmin: Story = {
  args: {
    isAdmin: false,
  },
};
