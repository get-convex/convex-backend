import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { CreateAdminKeyModal } from "./CreateAdminKeyModal";

describe("CreateAdminKeyModal", () => {
  it("calls onCreate and shows the key once", async () => {
    const onCreate = jest
      .fn()
      .mockResolvedValue({ adminKey: "prod:XYZ", id: "a", name: "CI", creationTime: 1 });
    const onClose = jest.fn();

    render(<CreateAdminKeyModal onCreate={onCreate} onClose={onClose} />);

    await userEvent.type(screen.getByLabelText(/name/i), "CI");
    fireEvent.click(screen.getByRole("button", { name: /create/i }));

    await waitFor(() => expect(onCreate).toHaveBeenCalledWith("CI"));
    expect(await screen.findByText(/prod:XYZ/)).toBeInTheDocument();
    expect(
      screen.getByText(/copy your new admin key now/i),
    ).toBeInTheDocument();
  });

  it("does not submit when name is empty", async () => {
    const onCreate = jest.fn();
    render(<CreateAdminKeyModal onCreate={onCreate} onClose={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /create/i }));
    expect(onCreate).not.toHaveBeenCalled();
  });
});
