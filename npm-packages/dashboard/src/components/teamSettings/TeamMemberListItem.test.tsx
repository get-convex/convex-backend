import {
  render,
  screen,
  fireEvent,
  act,
  waitFor,
} from "@testing-library/react";
import {
  Team,
  ProjectDetails,
  MemberResponse,
  TeamMemberResponse,
} from "generatedApi";
import { TeamMemberListItem } from "./TeamMemberListItem";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));
jest.mock("api/profile", () => {});
jest.mock("api/teams", () => {});
jest.mock("api/projects", () => {});
jest.mock("api/deployments", () => {});
jest.mock("api/roles", () => ({
  useHasProjectAdminPermissions: jest.fn(),
}));

describe("TeamMemberListItem", () => {
  const team: Team = {
    id: 1,
    name: "Team A",
    creator: 0,
    slug: "",
    suspended: false,
  };
  const projects: ProjectDetails[] = [
    {
      id: 1,
      name: "Project 1",
      slug: "project-1",
      teamId: 1,
      isDemo: false,
      createTime: 0,
    },
    {
      id: 2,
      name: "Project 2",
      slug: "project-2",
      teamId: 1,
      isDemo: false,
      createTime: 0,
    },
  ];
  const myProfile: MemberResponse = {
    id: 1,
    name: "John Doe",
    email: "",
  };
  const member: TeamMemberResponse = {
    id: 2,
    name: "Jane Smith",
    email: "",
    role: "developer",
  };
  const admin: TeamMemberResponse = {
    id: 3,
    name: "Admin User",
    email: "",
    role: "admin",
  };

  const members = [member, admin];
  const onChangeRole = jest.fn();
  const onRemoveMember = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  test("renders member name", () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={myProfile}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole={false}
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions
      />,
    );
    const memberName = screen.getByText(member.name!);
    expect(memberName).toBeInTheDocument();
  });

  test("does not render role combobox when canChangeRole is false", () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={myProfile}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole={false}
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions
      />,
    );
    const roleCombobox = screen.queryByTestId(`combobox-button-Role`);
    expect(roleCombobox).not.toBeInTheDocument();

    // Still renders the role
    screen.getByText("Developer");
  });

  test("disables role combobox when hasAdminPermissions is false", () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={myProfile}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions={false}
      />,
    );
    const roleCombobox = screen.queryByTestId(`combobox-button-Role`);
    expect(roleCombobox).not.toBeInTheDocument();

    // Still renders the role
    screen.getByText("Developer");
  });

  test("calls onChangeRole when role is changed", async () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={myProfile}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions
      />,
    );
    act(() => {
      const roleCombobox = screen.getByTestId(`combobox-button-Role`);
      fireEvent.click(roleCombobox);
    });
    await waitFor(() => expect(onChangeRole).not.toHaveBeenCalled());
    act(() => {
      const roleOption = screen.getByText("Admin");
      fireEvent.click(roleOption);
    });
    await waitFor(() =>
      expect(onChangeRole).toHaveBeenCalledWith({
        memberId: member.id,
        role: "admin",
      }),
    );
  });

  test("disables remove button when hasAdminPermissions is false", () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={myProfile}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions={false}
      />,
    );
    const removeButton = screen.getByText("Remove member");
    expect(removeButton).toBeDisabled();
  });

  test("calls onRemoveMember when remove button is clicked", async () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={myProfile}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions
      />,
    );
    const removeButton = screen.getByText("Remove member");
    expect(removeButton).toBeEnabled();
    await act(() => {
      fireEvent.click(removeButton);
    });

    expect(onRemoveMember).not.toHaveBeenCalled();
    await act(() => {
      const confirmButton = screen.getByText("Confirm");
      fireEvent.click(confirmButton);
    });

    expect(onRemoveMember).toHaveBeenCalledWith({ memberId: member.id });
  });

  test("renders leave team button when member is me", () => {
    render(
      <TeamMemberListItem
        team={team}
        myProfile={member}
        projects={projects}
        projectRoles={[]}
        onUpdateProjectRoles={jest.fn()}
        member={member}
        members={members}
        canChangeRole
        onChangeRole={onChangeRole}
        onRemoveMember={onRemoveMember}
        hasAdminPermissions
      />,
    );
    const leaveButton = screen.getByText("Leave team");
    expect(leaveButton).toBeInTheDocument();
  });
});
