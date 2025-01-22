import { render, screen, fireEvent } from "@testing-library/react";
import { InvitationResponse } from "generatedApi";
import { TeamMemberInviteListItem } from "./TeamMemberInviteListItem";

describe("TeamMemberInviteListItem", () => {
  const invite: InvitationResponse = {
    email: "test@example.com",
    expired: false,
    role: "developer",
  };

  const onCreateInvite = jest.fn();
  const onCancelInvite = jest.fn();

  it("renders invite details correctly", () => {
    render(
      <TeamMemberInviteListItem
        invite={invite}
        hasAdminPermissions
        onCreateInvite={onCreateInvite}
        onCancelInvite={onCancelInvite}
      />,
    );

    // Assert that invite details are rendered correctly
    expect(screen.getByText(invite.email)).toBeInTheDocument();
    expect(screen.queryByText("Invitation expired")).not.toBeInTheDocument();
    expect(screen.queryByText("Developer")).toBeInTheDocument();
  });

  it("renders invite details for admin invite", () => {
    render(
      <TeamMemberInviteListItem
        invite={{ ...invite, role: "admin" }}
        hasAdminPermissions
        onCreateInvite={onCreateInvite}
        onCancelInvite={onCancelInvite}
      />,
    );

    // Assert that invite details are rendered correctly
    expect(screen.getByText(invite.email)).toBeInTheDocument();
    expect(screen.queryByText("Invitation expired")).not.toBeInTheDocument();
    expect(screen.queryByText("Admin")).toBeInTheDocument();
  });

  it("renders expired invite details correctly", () => {
    render(
      <TeamMemberInviteListItem
        invite={{ ...invite, expired: true }}
        hasAdminPermissions
        onCreateInvite={onCreateInvite}
        onCancelInvite={onCancelInvite}
      />,
    );

    // Assert that invite details are rendered correctly
    expect(screen.getByText(invite.email)).toBeInTheDocument();
    expect(screen.getByText("Invitation expired")).toBeInTheDocument();
  });

  it("calls onCreateInvite when 'Resend' button is clicked", () => {
    render(
      <TeamMemberInviteListItem
        invite={invite}
        hasAdminPermissions
        onCreateInvite={onCreateInvite}
        onCancelInvite={onCancelInvite}
      />,
    );

    const resendButton = screen.getByText("Resend");
    fireEvent.click(resendButton);

    // Assert that onCreateInvite is called with the correct arguments
    expect(onCreateInvite).toHaveBeenCalledWith({
      email: invite.email,
      role: invite.role,
    });
  });

  it("calls onCancelInvite when 'Cancel' button is clicked", () => {
    render(
      <TeamMemberInviteListItem
        invite={invite}
        hasAdminPermissions
        onCreateInvite={onCreateInvite}
        onCancelInvite={onCancelInvite}
      />,
    );

    const cancelButton = screen.getByText("Revoke");
    fireEvent.click(cancelButton);

    // Assert that onCancelInvite is called with the correct arguments
    expect(onCancelInvite).toHaveBeenCalledWith({
      email: invite.email,
    });
  });
});
