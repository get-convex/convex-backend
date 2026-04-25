import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { RevokeAdminKeyButton } from "./RevokeAdminKeyModal";

describe("RevokeAdminKeyButton", () => {
  it("shows special warning when revoking the current key", async () => {
    const onRevoke = jest.fn().mockResolvedValue(undefined);
    render(<RevokeAdminKeyButton id="a" isCurrent onRevoke={onRevoke} />);
    fireEvent.click(screen.getByRole("button", { name: /revoke/i }));
    expect(
      await screen.findByText(
        /revoking this key will immediately log you out/i,
      ),
    ).toBeInTheDocument();
    // Headless UI marks the trigger button inert while the modal is open,
    // so only the modal's confirm button matches the /^revoke$/i query.
    const revokeButtons = screen.getAllByRole("button", { name: /^revoke$/i });
    fireEvent.click(revokeButtons[revokeButtons.length - 1]);
    await waitFor(() => expect(onRevoke).toHaveBeenCalledWith("a", true));
  });

  it("shows plain confirmation for non-current keys", async () => {
    const onRevoke = jest.fn().mockResolvedValue(undefined);
    render(
      <RevokeAdminKeyButton id="b" isCurrent={false} onRevoke={onRevoke} />,
    );
    fireEvent.click(screen.getByRole("button", { name: /revoke/i }));
    expect(screen.queryByText(/log you out/i)).not.toBeInTheDocument();
  });
});
