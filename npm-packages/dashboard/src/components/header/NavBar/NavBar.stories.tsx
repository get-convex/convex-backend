import { Meta, StoryObj } from "@storybook/nextjs";
import { NavBar } from "./NavBar";

const meta = { component: NavBar } satisfies Meta<typeof NavBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
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
