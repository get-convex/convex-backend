import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { AdminKeysList } from "./AdminKeysList";

const rows = [
  {
    id: "a",
    name: "laptop",
    creationTime: Date.UTC(2026, 3, 25),
    revokedTime: null,
    isCurrent: true,
    keySuffix: "Ab12Cd34",
  },
  {
    id: "b",
    name: "CI",
    creationTime: Date.UTC(2026, 3, 20),
    revokedTime: Date.UTC(2026, 3, 24),
    isCurrent: false,
    keySuffix: null,
  },
];

describe("AdminKeysList", () => {
  it("renders rows with the current-key badge, a revoked timestamp, and the key suffix", () => {
    render(
      <AdminKeysList keys={rows} onRevoke={jest.fn()} onRename={jest.fn()} />,
    );
    expect(screen.getByText("laptop")).toBeInTheDocument();
    expect(screen.getByText("CI")).toBeInTheDocument();
    expect(screen.getByText(/this key/i)).toBeInTheDocument();
    // Revoked timestamp prefix is rendered for revoked rows.
    expect(screen.getAllByText(/Revoked/i).length).toBeGreaterThan(0);
    // Key suffix shown only for the row that has one.
    expect(screen.getByText(/····Ab12Cd34/)).toBeInTheDocument();
  });

  it("shows the empty state when there are no keys", () => {
    render(
      <AdminKeysList keys={[]} onRevoke={jest.fn()} onRename={jest.fn()} />,
    );
    expect(
      screen.getByText(/there are no admin keys yet/i),
    ).toBeInTheDocument();
  });

  it("revokes from the kebab menu, with a current-key warning", async () => {
    const onRevoke = jest.fn().mockResolvedValue(undefined);
    render(
      <AdminKeysList
        keys={[rows[0]]}
        onRevoke={onRevoke}
        onRename={jest.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /admin key options/i }));
    fireEvent.click(await screen.findByRole("menuitem", { name: /revoke/i }));
    expect(
      await screen.findByText(/will immediately log you out/i),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /^revoke$/i }));
    await waitFor(() => expect(onRevoke).toHaveBeenCalledWith("a", true));
  });

  it("does not show the kebab menu on revoked rows", () => {
    render(
      <AdminKeysList
        keys={[rows[1]]}
        onRevoke={jest.fn()}
        onRename={jest.fn()}
      />,
    );
    expect(
      screen.queryByRole("button", { name: /admin key options/i }),
    ).not.toBeInTheDocument();
  });

  it("renames from the kebab menu", async () => {
    const onRename = jest.fn().mockResolvedValue(undefined);
    render(
      <AdminKeysList
        keys={[rows[0]]}
        onRevoke={jest.fn()}
        onRename={onRename}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /admin key options/i }));
    fireEvent.click(await screen.findByRole("menuitem", { name: /rename/i }));
    const input = await screen.findByDisplayValue("laptop");
    await userEvent.clear(input);
    await userEvent.type(input, "macbook");
    fireEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() => expect(onRename).toHaveBeenCalledWith("a", "macbook"));
  });
});
