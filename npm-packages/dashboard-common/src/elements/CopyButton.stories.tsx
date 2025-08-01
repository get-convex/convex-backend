import { Meta, StoryObj } from "@storybook/nextjs";
import { CopyButton } from "@common/elements/CopyButton";

const meta = { component: CopyButton } satisfies Meta<typeof CopyButton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    text: "Hello, world!",
  },
};
