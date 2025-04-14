import { Meta, StoryObj } from "@storybook/react";
import { Menu, MenuItem, MenuLink } from "@ui/Menu";

export default {
  component: Menu,
  render: (args) => (
    <div className="w-fit">
      <Menu {...args} />
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
    children: (
      <>
        <MenuItem action={() => {}} shortcut={["CtrlOrCmd", "C"]}>
          Item 1
        </MenuItem>
        <MenuItem action={() => {}} variant="danger">
          Item 2
        </MenuItem>
        <MenuLink href="/blah" shortcut={["CtrlOrCmd", "O"]}>
          Item 3 (Link)
        </MenuLink>
      </>
    ),
  },
};
