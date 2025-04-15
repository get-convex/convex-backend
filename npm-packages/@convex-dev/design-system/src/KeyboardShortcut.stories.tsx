import { Meta, StoryObj } from "@storybook/react";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";

const meta: Meta<typeof KeyboardShortcut> = {
  component: KeyboardShortcut,
};

export default meta;

export const Windows: StoryObj<typeof KeyboardShortcut> = {
  args: {
    isApple: false,
    value: ["Ctrl", "Alt", "Delete"],
  },
};

export const Apple: StoryObj<typeof KeyboardShortcut> = {
  args: {
    isApple: true,
    value: ["CtrlOrCmd", "Alt", "Shift", "Return"],
  },
};

export const AutoDetect: StoryObj<typeof KeyboardShortcut> = {
  args: {
    value: ["CtrlOrCmd", "C"],
  },
};
