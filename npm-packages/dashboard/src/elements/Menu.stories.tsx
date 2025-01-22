import { Meta, StoryObj } from "@storybook/react";
import { toast } from "dashboard-common";
import { ToastContainer } from "elements/ToastContainer";
import { Menu, MenuItem, MenuLink } from "./Menu";

export default {
  component: Menu,
  render: (args) => (
    <div className="w-fit">
      <Menu {...args} />
      <ToastContainer />
    </div>
  ),
} as Meta<typeof Menu>;

export const Primary: StoryObj<typeof Menu> = {
  args: {
    buttonProps: {
      children: "Menu",
      variant: "neutral",
    },
    placement: "bottom",
    children: [
      <MenuItem
        action={() => toast("success", "Item 1 clicked!")}
        shortcut={["CtrlOrCmd", "C"]}
      >
        Item 1
      </MenuItem>,
      <MenuItem
        action={() => toast("success", "Item 2 clicked!")}
        variant="danger"
      >
        Item 2
      </MenuItem>,
      <MenuLink href="/blah" shortcut={["CtrlOrCmd", "O"]}>
        Item 3 (Link)
      </MenuLink>,
    ],
  },
};
