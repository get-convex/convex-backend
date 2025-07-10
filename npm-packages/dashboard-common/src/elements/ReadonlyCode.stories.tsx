import { Meta, StoryObj } from "@storybook/nextjs";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";

export default {
  component: ReadonlyCode,
  args: {
    language: "javascript",
    path: "example.js",
  },
  render: (args) => (
    <div style={{ height: "100vh", background: "#eee" }}>
      <ReadonlyCode {...args} />
    </div>
  ),
} as Meta<typeof ReadonlyCode>;
export const ParentHeight: StoryObj<typeof ReadonlyCode> = {
  args: {
    code: 'console.log("Hello World");',
  },
};

export const ContentHeight: StoryObj<typeof ReadonlyCode> = {
  args: {
    language: "python",
    code: 'for i in range(10):\n\tprint("Hello world")',
    path: "example.py",
    height: { type: "content" },
  },
};

export const ContentHeightWithMax: StoryObj<typeof ReadonlyCode> = {
  args: {
    code: 'console.log("Hello World");\n'.repeat(100),
    height: {
      type: "content",
      maxHeightRem: 20,
    },
  },
};

export const HighlightLines: StoryObj<typeof ReadonlyCode> = {
  args: {
    code: 'console.log("Hello World");\n'.repeat(100),
    highlightLines: {
      startLineNumber: 10,
      endLineNumber: 20,
    },
  },
};
