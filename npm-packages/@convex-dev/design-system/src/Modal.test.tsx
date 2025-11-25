import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Modal } from "./Modal";

describe("Modal", () => {
  const defaultProps = {
    onClose: jest.fn(),
    title: "Test Title",
    description: "Test Description",
    children: <div>Modal Content</div>,
  };

  it("closes and triggers onClose after ClosePanelButton is clicked", async () => {
    const user = userEvent.setup();
    const onClose = jest.fn();
    render(<Modal {...defaultProps} onClose={onClose} />);
    const closeButton = screen.getByRole("button");
    await user.click(closeButton);
    // Wait for afterLeave/onClose after transition
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
      expect(screen.queryByTestId("modal")).not.toBeInTheDocument();
    });
  });

  it("closes and triggers onClose after modal overlay is clicked", async () => {
    const user = userEvent.setup();
    const onClose = jest.fn();
    render(<Modal {...defaultProps} onClose={onClose} />);
    const overlay = screen.getByTestId("modal-overlay");
    await user.click(overlay);
    // Wait for afterLeave/onClose after transition
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
      expect(screen.queryByTestId("modal")).not.toBeInTheDocument();
    });
  });
});
