import type { Meta, StoryObj } from "@storybook/nextjs";
import { RadioGroup } from "@headlessui/react";
import { Region } from "./Region";

const meta = {
  component: Region,
  render: (args) => (
    <RadioGroup defaultValue="aws-us-east-1">
      <Region {...args} />
    </RadioGroup>
  ),
  args: {
    teamSlug: "example-team",
  },
} satisfies Meta<typeof Region>;

export default meta;
type Story = StoryObj<typeof meta>;

export const USEast: Story = {
  args: {
    region: {
      displayName: "US East (N. Virginia)",
      name: "aws-us-east-1",
      available: true,
    },
  },
};

export const Europe: Story = {
  args: {
    region: {
      displayName: "Europe (Ireland)",
      name: "aws-eu-west-1",
      available: true,
    },
  },
};

export const UnavailableRegion: Story = {
  args: {
    region: {
      displayName: "Europe (Ireland)",
      name: "aws-eu-west-1",
      available: false,
    },
  },
};

export const Local: Story = {
  args: {
    region: {
      displayName: "test",
      name: "local",
      available: true,
    },
  },
};

export const AskEveryTime: Story = {
  args: {
    region: null,
  },
};

export const Loading: Story = {
  args: {
    region: undefined,
  },
};

export const DisabledDueToPermissions: Story = {
  args: {
    region: {
      displayName: "US East (N. Virginia)",
      name: "aws-us-east-1",
      available: true,
    },
    disabledDueToPermissions: true,
  },
};
