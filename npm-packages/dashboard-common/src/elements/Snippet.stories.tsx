import { Meta, StoryObj } from "@storybook/nextjs";
import { Snippet } from "@common/elements/Snippet";

const meta = {
  component: Snippet,
  render: (args) => (
    <div style={{ maxWidth: 300 }}>
      <Snippet {...args} />
    </div>
  ),
} satisfies Meta<typeof Snippet>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    value: "Something you can copy",
    copying: "something",
  },
};

export const Truncated: Story = {
  args: {
    value: "Something longer that you can't read all of, but you can still",
    copying: "something",
  },
};

export const Code: Story = {
  args: {
    value: "console.log('Hello world');",
    monospace: true,
    copying: "something code-like",
  },
};
