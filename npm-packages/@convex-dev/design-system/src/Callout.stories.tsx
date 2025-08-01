import { Meta, StoryObj } from "@storybook/nextjs";
import { Callout } from "@ui/Callout";

const meta = { component: Callout } satisfies Meta<typeof Callout>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Error: Story = {
  args: {
    variant: "error",
    children: "This is an error",
  },
};

export const Instructions: Story = {
  args: {
    children: "These are instructions",
  },
};
