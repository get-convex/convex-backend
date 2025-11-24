import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { TeamResponse } from "generatedApi";
import { InviteMemberForm, InviteMemberFormProps } from "./InviteMemberForm";

jest.mock("api/roles", () => ({
  useHasProjectAdminPermissions: jest.fn(),
}));
jest.mock("api/profile", () => {});
jest.mock("api/teams", () => {});
jest.mock("api/projects", () => {});
jest.mock("api/deployments", () => {});
jest.mock("api/backups", () => {});
jest.mock("api/billing", () => ({
  useTeamOrbSubscription: jest.fn().mockReturnValue({ subscription: null }),
}));

jest.mock("api/invitations", () => {
  const createInvite = jest.fn();
  const useCreateInvite = jest.fn().mockReturnValue(createInvite);
  return { useCreateInvite };
});

const { useCreateInvite } = require("../../api/invitations");
// eslint-disable-next-line react-hooks/rules-of-hooks
const createInvite = useCreateInvite();

describe("<InviteMemberForm />", () => {
  let team: TeamResponse;
  let emailTextbox: HTMLInputElement;

  const setup = (props?: Partial<InviteMemberFormProps>) => {
    render(
      <InviteMemberForm
        team={team}
        members={[]}
        hasAdminPermissions
        {...props}
      />,
    );

    emailTextbox = screen.getByRole("textbox", {
      name: /Email address/i,
    });
  };

  beforeEach(() => {
    jest.clearAllMocks();
    team = {
      id: 1,
      creator: 1,
      name: "Convex team",
      slug: "convex-team",
      suspended: false,
      referralCode: "CODE123",
    };
  });

  it("should load form", () => {
    setup();
    const form = screen.getByRole("form", {
      name: /Invite team member/i,
    });
    expect(form).toHaveFormValues({
      inviteEmail: "",
    });

    expect(screen.getByText("Send Invite")).toBeDisabled();
  });

  it("should accept input and save with developer role", async () => {
    setup({});
    const user = userEvent.setup();
    await user.type(emailTextbox, "ari@convex.dev");
    expect(emailTextbox).toHaveValue("ari@convex.dev");

    expect(screen.getByText("Send Invite")).toBeEnabled();

    await user.click(screen.getByText("Send Invite"));
    expect(useCreateInvite).toHaveBeenCalledWith(team.id);
    expect(createInvite).toHaveBeenCalledWith({
      email: "ari@convex.dev",
      role: "developer",
    });
  });

  it("should accept input and save with admin role", async () => {
    setup({});
    const user = userEvent.setup();
    await user.type(emailTextbox, "ari@convex.dev");
    expect(emailTextbox).toHaveValue("ari@convex.dev");

    expect(screen.getByText("Send Invite")).toBeEnabled();

    const roleCombobox = screen.getByTestId("combobox-button-Role");
    await user.click(roleCombobox);

    const roleOption = screen.getByText("Admin");
    await user.click(roleOption);

    await user.click(screen.getByText("Send Invite"));
    expect(useCreateInvite).toHaveBeenCalledWith(team.id);
    expect(createInvite).toHaveBeenCalledWith({
      email: "ari@convex.dev",
      role: "admin",
    });
  });

  it("should not allow inviting existing member", async () => {
    setup({ members: [{ id: 1, email: "ari@convex.dev", role: "developer" }] });

    const user = userEvent.setup();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.type(emailTextbox, "ari@convex.dev");
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Send Invite")).toBeDisabled();
  });

  it("should disable when not an admin", () => {
    setup({ hasAdminPermissions: false });
    expect(screen.getByLabelText("Email address")).toBeDisabled();
    expect(
      screen.queryByLabelText("combobox-button-Role"),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Send Invite")).toBeDisabled();
  });

  it("should display email validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.type(emailTextbox, "ari@");
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Send Invite")).toBeDisabled();
  });
});
