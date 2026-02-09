import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { TeamResponse } from "generatedApi";
import userEvent from "@testing-library/user-event";
import * as deployments from "api/deployments";
import { TeamForm, TeamFormProps } from "./TeamForm";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));
jest.mock("api/deployments");

// Mock out location to prevent
// Error: Not implemented: navigation (except hash changes)
// In Jest 30 with JSDOM v22, window.location is non-configurable,
// so we define it on Window.prototype instead
Object.defineProperty(Window.prototype, "location", {
  get: () => ({
    href: "",
    assign: jest.fn(),
    replace: jest.fn(),
    reload: jest.fn(),
  }),
  configurable: true,
});

describe("<TeamForm />", () => {
  let team: TeamResponse;
  let updatedTeam: TeamResponse;
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
    jest.spyOn(deployments, "useDeploymentRegions").mockReturnValue({
      regions: undefined,
      isLoading: false,
    });
    team = {
      id: 1,
      creator: 1,
      name: "Convex team",
      slug: "convex-team",
      suspended: false,
      referralCode: "CODE123",
    };
    updatedTeam = {
      id: 1,
      creator: 1,
      name: "Convex team 2",
      slug: "convex-team2",
      suspended: false,
      referralCode: "CODE123",
    };
  });

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
      defaultRegion: null,
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

  test("should allow changing the default region", async () => {
    jest.spyOn(deployments, "useDeploymentRegions").mockReturnValue({
      regions: [
        {
          name: "aws-us-east-1",
          displayName: "US East (N. Virginia)",
          available: true,
        },
        {
          name: "aws-eu-west-1",
          displayName: "EU West (Ireland)",
          available: true,
        },
      ],
      isLoading: false,
    });
    setup();
    const user = userEvent.setup();

    // Verify region selector is shown
    expect(screen.getByText("Region for New Deployments")).toBeInTheDocument();

    // Select US East region
    await user.click(screen.getByText("US East"));

    expect(screen.getByText("Save")).toBeEnabled();

    await act(async () => {
      fireEvent.click(screen.getByText("Save"));
    });
    expect(onUpdateTeam).toHaveBeenCalledWith({
      name: team.name,
      slug: team.slug,
      defaultRegion: "aws-us-east-1",
    });
  });
});
