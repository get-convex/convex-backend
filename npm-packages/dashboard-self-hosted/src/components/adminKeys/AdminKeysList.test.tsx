import { render, screen } from "@testing-library/react";

import { AdminKeysList } from "./AdminKeysList";

const rows = [
  {
    id: "a",
    name: "laptop",
    creationTime: Date.UTC(2026, 3, 25),
    revokedTime: null,
    isCurrent: true,
  },
  {
    id: "b",
    name: "CI",
    creationTime: Date.UTC(2026, 3, 20),
    revokedTime: Date.UTC(2026, 3, 24),
    isCurrent: false,
  },
];

describe("AdminKeysList", () => {
  it("renders rows with status and current-key badge", () => {
    render(
      <AdminKeysList keys={rows} onRevoke={jest.fn()} onRename={jest.fn()} />,
    );
    expect(screen.getByText("laptop")).toBeInTheDocument();
    expect(screen.getByText(/this key/i)).toBeInTheDocument();
    expect(screen.getByText("Revoked")).toBeInTheDocument();
  });
});
