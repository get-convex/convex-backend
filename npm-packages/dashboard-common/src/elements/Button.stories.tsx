import { Meta, StoryObj } from "@storybook/react";
import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "./Button";

export default {
  component: Button,
  render: (args: any) => <Button {...args} />,
} as Meta<typeof Button>;

export const Primary: StoryObj<typeof Button> = {
  args: {
    variant: "primary",
    icon: <PlusIcon />,
    children: "Button",
  },
};

export const PrimaryFocused: StoryObj<typeof Button> = {
  args: {
    variant: "primary",
    focused: true,
    children: "Button",
  },
};

export const Destructive: StoryObj<typeof Button> = {
  args: {
    variant: "danger",
    children: "Button",
  },
};

export const DestructiveFocused: StoryObj<typeof Button> = {
  args: {
    variant: "danger",
    focused: true,
    children: "Button",
  },
};

export const Disabled: StoryObj<typeof Button> = {
  args: {
    disabled: true,
    children: "Button",
  },
};

export const Link: StoryObj<typeof Button> = {
  args: {
    href: "https://convex.dev",
    children: "Link button",
  },
};

export const LinkDisabled: StoryObj<typeof Button> = {
  args: {
    href: "https://convex.dev",
    disabled: true,
    children: "No link because it's disabled",
  },
};

export const LinkTargetBlack: StoryObj<typeof Button> = {
  args: {
    href: "https://convex.dev",
    target: "_blank",
    children: "Open in a new tab",
  },
};

export const LinkOnClick: StoryObj<typeof Button> = {
  args: {
    href: "https://convex.dev",
    onClick: () => {
      // eslint-disable-next-line no-alert
      alert("When you dismiss you'll be redirected");
    },
    children: "Redirect after alert",
  },
};

export const LinkTip: StoryObj<typeof Button> = {
  args: {
    href: "https://convex.dev",
    children: "Link button",
    tip: "Goes somewhere",
  },
};
