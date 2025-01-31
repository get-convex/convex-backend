import { Meta, StoryObj } from "@storybook/react";
import {
  ArrowUpIcon,
  FrameIcon,
  Pencil1Icon,
  PlusIcon,
  PersonIcon,
  Cross1Icon,
} from "@radix-ui/react-icons";
import { Sidebar } from "elements/Sidebar";

export default {
  component: Sidebar,
  render: (args) => (
    <div className="m-[-1rem] h-[100vh] w-[100vw] bg-background-primary">
      <Sidebar {...args} />
    </div>
  ),
} as Meta<typeof Sidebar>;

export const Primary: StoryObj<typeof Sidebar> = {
  args: {
    collapsed: false,
    items: [
      {
        key: "group1",
        items: [
          {
            key: "A",
            label: "Item 1",
            Icon: PlusIcon,
            href: "/a",
          },
          {
            key: "B",
            label: "Item 2",
            Icon: Cross1Icon,
            href: "/b",
          },
          {
            key: "C",
            label: "Item 3",
            Icon: ArrowUpIcon,
            href: "/c",
          },
        ],
      },
      {
        key: "group2",
        items: [
          {
            key: "D",
            label: "Item 4",
            Icon: PersonIcon,
            href: "/",
          },
          {
            key: "E",
            label: "Item 5",
            Icon: FrameIcon,
            href: "/e",
          },
        ],
      },
      {
        key: "group3",
        items: [
          {
            key: "F",
            label: "Item 6",
            Icon: Pencil1Icon,
            href: "/f",
          },
        ],
      },
    ],
  },
};
