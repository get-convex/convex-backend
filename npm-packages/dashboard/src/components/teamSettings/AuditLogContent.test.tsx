import { render, screen } from "@testing-library/react";
import {
  MemberResponse,
  Team,
  ProjectDetails,
  AuditLogEventResponse,
} from "generatedApi";
import { AuditLogContent } from "./AuditLogContent";

jest.mock("api/backups", () => {});
jest.mock("api/profile", () => {});
jest.mock("api/teams", () => {});
jest.mock("api/projects", () => {});
jest.mock("api/deployments", () => {});
jest.mock("api/backups", () => {});

jest.mock("../../elements/TeamMemberLink", () => ({
  __esModule: true,
  TeamMemberLink: jest.fn().mockReturnValue(<div>Mocked TeamMemberLink</div>),
}));

describe("AuditLogContent", () => {
  const team: Team = {
    id: 1,
    name: "Team 1",
    slug: "team-1",
    creator: 1,
    suspended: false,
    referralCode: "CODE123",
    referralVerified: false,
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
  const members: MemberResponse[] = [
    { id: 1, name: "Member 1", email: "" },
    { id: 2, name: "Member 2", email: "" },
  ];
  const entries: AuditLogEventResponse[] = [
    {
      action: "createTeam",
      teamId: 1,
      createTime: new Date("2022-01-01T00:00:00Z").getTime(),
      actor: { member: { member_id: 1 } },
      metadata: {
        noun: "team",
        current: team,
      },
    },
    {
      action: "updateTeam",
      teamId: 1,
      createTime: new Date("2022-01-02T00:00:00Z").getTime(),
      actor: { member: { member_id: 1 } },
      metadata: {
        noun: "team",
        previous: team,
        current: { ...team, name: "Team Fun Time" },
      },
    },
  ];

  it("renders the audit log content correctly", () => {
    render(
      <AuditLogContent
        team={team}
        projects={projects}
        members={members}
        entries={entries}
      />,
    );

    expect(screen.getAllByTestId("audit-log-item")).toHaveLength(2);
  });

  it("renders 'NoEntries' component when there are no audit log entries", () => {
    render(
      <AuditLogContent
        team={team}
        projects={projects}
        members={members}
        entries={[]}
      />,
    );

    expect(screen.getByTestId("no-entries")).toBeInTheDocument();
  });
});
