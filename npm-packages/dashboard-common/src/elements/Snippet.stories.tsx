import { Meta, StoryObj } from "@storybook/nextjs";
import { Snippet } from "@common/elements/Snippet";

export default {
  component: Snippet,
  render: (args) => (
    <div style={{ maxWidth: 300 }}>
      <Snippet {...args} value="Something you can copy" />
      <br />
      <Snippet
        {...args}
        value="Something longer that you can't read all of, but you can still"
      />
    </div>
  ),
} as Meta<typeof Snippet>;

export const Primary: StoryObj<typeof Snippet> = {
  args: {
    copying: "something",
  },
};

export const Code: StoryObj<typeof Snippet> = {
  args: {
    monospace: true,
    copying: "something code-like",
  },
};
