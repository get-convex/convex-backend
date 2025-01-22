import { act, render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Menu, MenuItem, MenuLink, MenuProps } from "./Menu";

describe("Menu", () => {
  beforeEach(jest.clearAllMocks);

  const setup = (props: Omit<MenuProps, "buttonProps">) =>
    render(
      <Menu
        buttonProps={{
          children: "Menu",
        }}
        {...props}
      />,
    );

  it("clicking a menu item calls the action and closes the menu", async () => {
    const action = jest.fn();
    const { getByRole, getByText, queryByText } = setup({
      children: <MenuItem action={action}>Item</MenuItem>,
    });

    const user = userEvent.setup();

    const button = getByRole("button");
    await user.click(button);

    const item = getByText("Item");
    await user.click(item);
    expect(action).toHaveBeenCalledTimes(1);

    expect(queryByText("Item")).toBeNull();
  });

  it("clicking a menu link redirects and closes the menu", async () => {
    const { getByRole, getByText } = setup({
      children: <MenuLink href="/blah">Item</MenuLink>,
    });

    const user = userEvent.setup();

    const button = getByRole("button");
    await act(() => user.click(button));

    const item = getByText("Item");
    expect(item).toHaveAttribute("href", "/blah");
  });
});
