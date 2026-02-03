import type { Meta, StoryObj } from "@storybook/nextjs";
import { fn } from "storybook/test";
import { Sheet } from "@ui/Sheet";
import { DefaultRegionSelector } from "./DefaultRegionSelector";

const meta = {
  component: DefaultRegionSelector,
  args: {
    value: null,
    onChange: fn(),
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
    teamSlug: "example-team",
  },
  decorators: [
    (Story) => (
      <Sheet>
        <Story />
      </Sheet>
    ),
  ],
} satisfies Meta<typeof DefaultRegionSelector>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const WithSelectedRegion: Story = {
  args: {
    value: "aws-us-east-1",
  },
};

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
  },
};

export const NonAdmin: Story = {
  args: {
    disabledDueToPermissions: true,
  },
};

export const NonAdminEuropeNotAvailable: Story = {
  args: {
    disabledDueToPermissions: true,
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
  },
};

export const Loading: Story = {
  args: {
    regions: undefined,
  },
};
