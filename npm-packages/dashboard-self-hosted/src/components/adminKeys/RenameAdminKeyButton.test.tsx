import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { RenameAdminKeyButton } from "./RenameAdminKeyButton";

it("submits new name", async () => {
  const onRename = jest.fn().mockResolvedValue(undefined);
  render(<RenameAdminKeyButton id="a" name="old" onRename={onRename} />);
  fireEvent.click(screen.getByRole("button", { name: /rename/i }));
  const input = await screen.findByDisplayValue("old");
  await userEvent.clear(input);
  await userEvent.type(input, "new");
  fireEvent.click(screen.getByRole("button", { name: /save/i }));
  await waitFor(() => expect(onRename).toHaveBeenCalledWith("a", "new"));
});
