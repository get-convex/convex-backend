import { Meta, StoryObj } from "@storybook/nextjs";
import { Avatar } from "./Avatar";

const meta = { component: Avatar } satisfies Meta<typeof Avatar>;

export default meta;
type Story = StoryObj<typeof Avatar>;

export const Initials: Story = {
  args: {
    name: "Zepp Williams",
  },
};
