import { Meta, StoryObj } from "@storybook/react";
import { ReadonlyCodeDiff } from "elements/ReadonlyCode";

export default {
  component: ReadonlyCodeDiff,
  args: {
    language: "javascript",
    path: "example.js",
  },
  render: (args) => (
    <div style={{ height: "100vh", background: "#eee" }}>
      <ReadonlyCodeDiff {...args} />
    </div>
  ),
} as Meta<typeof ReadonlyCodeDiff>;

export const ParentHeight: StoryObj<typeof ReadonlyCodeDiff> = {
  args: {
    originalCode:
      "This line is removed on the right.\njust some text\nabcd\nefgh\nSome more text",
    modifiedCode:
      "just some text\nabcz\nzzzzefgh\nSome more text.\nThis line is removed on the left.",
  },
};

export const ContentHeight: StoryObj<typeof ReadonlyCodeDiff> = {
  args: {
    language: "plaintext",
    originalCode:
      "Hello world\nHello world\nHello world\nHello world\nHello world\nHello world\nHelloworld\nHello world\n",
    modifiedCode:
      "Hi world\nHello world\nHello world\nHello world\nHello world\nHello world\nHello world\nHello  world\nHello world\nHello world",
    height: { type: "content" },
  },
};
