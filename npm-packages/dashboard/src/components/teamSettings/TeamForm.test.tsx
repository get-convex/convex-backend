import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { Team } from "generatedApi";
import userEvent from "@testing-library/user-event";
import { TeamForm, TeamFormProps } from "./TeamForm";

const locationMock = jest.fn();

// Mock out location to prevent
// Error: Not implemented: navigation (except hash changes)
delete (window as any).location;
window.location = locationMock as unknown as Location;

describe("<TeamForm />", () => {
  let team: Team;
  let updatedTeam: Team;
  let nameTextBox: HTMLInputElement;
  let slugTextBox: HTMLInputElement;
  const onUpdateTeam = jest.fn();

  const setup = (props?: Partial<TeamFormProps>) => {
    render(
      <TeamForm
        team={team}
        onUpdateTeam={onUpdateTeam}
        hasAdminPermissions
        {...props}
      />,
    );

    nameTextBox = screen.getByRole("textbox", {
      name: /name/i,
    });
    slugTextBox = screen.getByRole("textbox", {
      name: /slug/i,
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
    };
    updatedTeam = {
      id: 1,
      creator: 1,
      name: "Convex team 2",
      slug: "convex-team2",
      suspended: false,
    };
  });

  afterEach(() => locationMock.mockClear());

  it("should load team into form", () => {
    setup();
    const form = screen.getByRole("form", {
      name: /Edit team settings/i,
    });
    expect(form).toHaveFormValues({
      name: team.name,
      slug: team.slug,
    });

    expect(screen.getByText("Save")).toBeDisabled();
  });

  test("should accept input and save", async () => {
    setup();
    const user = userEvent.setup();
    // Super annoying: we have some version incompatibilities that make userEvent
    // not apply act() correctly.
    // Once we upgrade to Next 18 we can remove these, but until then we need to
    // manually wrap these in act.
    await user.clear(nameTextBox);
    await user.type(nameTextBox, updatedTeam.name);
    expect(nameTextBox).toHaveValue(updatedTeam.name);

    await user.clear(slugTextBox);
    await user.type(slugTextBox, updatedTeam.slug);
    expect(slugTextBox).toHaveValue(updatedTeam.slug);

    expect(screen.getByText("Save")).toBeEnabled();

    await act(async () => {
      fireEvent.click(screen.getByText("Save"));
    });
    expect(onUpdateTeam).toHaveBeenCalledWith({
      name: updatedTeam.name,
      slug: updatedTeam.slug,
    });
  });

  test("name should display required validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(nameTextBox);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Save")).toBeDisabled();
  });

  test("name should display minlength validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(nameTextBox);
    await user.type(nameTextBox, "ab");
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Team name must be at least 3 characters long.",
    );
    expect(screen.getByText("Save")).toBeDisabled();

    await user.type(nameTextBox, "c");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByText("Save")).toBeEnabled();
  });

  test("name should display maxlength validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(nameTextBox);
    await user.type(nameTextBox, "a".repeat(128));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.type(nameTextBox, "b");
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Team name must be at most 128 characters long.",
    );
    expect(screen.getByText("Save")).toBeDisabled();
  });

  test("name should display required validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(nameTextBox);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Save")).toBeDisabled();
  });

  test("slug should display minlength validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(slugTextBox);
    await user.type(slugTextBox, "ab");
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Team slug must be at least 3 characters long.",
    );
    expect(screen.getByText("Save")).toBeDisabled();

    await user.type(slugTextBox, "c");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByText("Save")).toBeEnabled();
  });

  test("slug should display maxlength validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(slugTextBox);
    await user.type(slugTextBox, "a".repeat(64));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.type(slugTextBox, "b");
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Team slug must be at most 64 characters long.",
    );
    expect(screen.getByText("Save")).toBeDisabled();
  });

  test("slug should display required validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(slugTextBox);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Save")).toBeDisabled();
  });

  test("slug should display invalid character validation", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(slugTextBox);
    await user.type(slugTextBox, " ".repeat(3));
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Team slug may contain numbers, letters, underscores, and '-'.",
    );
    expect(screen.getByText("Save")).toBeDisabled();
  });

  it("should disable inputs when user does not have admin permissions", () => {
    setup({ hasAdminPermissions: false });

    expect(nameTextBox).toBeDisabled();
    expect(slugTextBox).toBeDisabled();
    expect(screen.getByText("Save")).toBeDisabled();
  });
});
