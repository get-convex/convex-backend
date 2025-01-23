import { StoryObj } from "@storybook/react";
import { NavBar } from "./NavBar";

export default { component: NavBar };

export const Primary: StoryObj<typeof NavBar> = {
  args: {
    activeLabel: "One",
    items: [
      {
        label: "One",
        href: "/one",
      },
      {
        label: "Two",
        href: "/two",
      },
      {
        label: "Three",
        href: "/three",
      },
    ],
  },
};
