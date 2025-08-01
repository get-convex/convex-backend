import { Meta, StoryObj } from "@storybook/nextjs";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";

const meta = {
  component: KeyboardShortcut,
} satisfies Meta<typeof KeyboardShortcut>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Windows: Story = {
  args: {
    isApple: false,
    value: ["Ctrl", "Alt", "Delete"],
  },
};

export const Apple: Story = {
  args: {
    isApple: true,
    value: ["CtrlOrCmd", "Alt", "Shift", "Return"],
  },
};

export const AutoDetect: Story = {
  args: {
    value: ["CtrlOrCmd", "C"],
  },
};
