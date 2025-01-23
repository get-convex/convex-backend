import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as nextRouter from "next/router";
// @ts-ignore `createTeam` is exported in the mock, but not in the real module.
import { useCreateTeam, createTeam } from "api/teams";
import { CreateTeamForm } from "./CreateTeamForm";

const mockRouter = jest
  .fn()
  .mockImplementation(() => ({ route: "/", push: jest.fn() }));
(nextRouter as any).useRouter = mockRouter;

jest.mock("api/teams", () => {
  const create = jest.fn();
  return {
    useCreateTeam: jest.fn().mockReturnValue(create),
    createTeam: create,
  };
});

describe("<CreateTeamForm />", () => {
  let nameTextBox: HTMLInputElement;

  const setup = () => {
    render(<CreateTeamForm onClose={jest.fn()} />);

    nameTextBox = screen.getByRole("textbox", {
      name: /Team name/i,
    });
  };

  test("should accept input and save", async () => {
    setup();
    const user = userEvent.setup();
    await user.clear(nameTextBox);
    await user.type(nameTextBox, "my team");
    expect(nameTextBox).toHaveValue("my team");

    expect(screen.getByText("Create")).toBeEnabled();

    await act(async () => {
      fireEvent.click(screen.getByText("Create"));
    });
    expect(useCreateTeam).toHaveBeenCalled();
    expect(createTeam).toHaveBeenCalledWith({ name: "my team" });
  });
});
