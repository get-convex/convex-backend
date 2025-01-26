import { render, screen } from "@testing-library/react";
import mockRouter from "next-router-mock";
import { endOfDay, parse, startOfDay } from "date-fns";
import { act } from "react";
import { DATE_FORMAT } from "dashboard-common";
import { AuditLog } from "./AuditLog";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));

jest.mock("api/backups", () => {});

const loadNextPage = jest.fn();
const useTeamAuditLog = jest.fn().mockReturnValue({
  entries: [],
  isLoading: false,
  loadNextPage,
  hasMore: true,
});
jest.mock("api/profile", () => {});
jest.mock("api/deployments", () => {});
jest.mock("api/projects", () => ({
  useProjects: () => [],
}));

jest.mock("../../hooks/api", () => ({
  useTeamAuditLog: (teamId: number, args: any) => useTeamAuditLog(teamId, args),
}));

jest.mock("api/teams", () => ({
  useTeamMembers: () => [],
}));

jest.mock("dashboard-common", () => ({
  ...jest.requireActual("dashboard-common"),
  DateRangePicker: jest.fn(),
}));

describe("AuditLog", () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("loads the correct default filters", () => {
    mockRouter.setCurrentUrl("/some-url");
    render(
      <AuditLog
        team={{
          id: 1,
          name: "Team 1",
          slug: "team-1",
          creator: 1,
          suspended: false,
        }}
      />,
    );
    expect(useTeamAuditLog).toHaveBeenCalledTimes(1);

    const currentDate = new Date();
    const sevenDaysAgo = new Date(
      currentDate.getTime() - 6 * 24 * 60 * 60 * 1000,
    );
    expect(useTeamAuditLog).toHaveBeenCalledWith(1, {
      from: startOfDay(sevenDaysAgo).getTime() - 1,
      to: endOfDay(currentDate).getTime(),
      memberId: null,
      action: null,
    });
  });

  it("loads the correct filters based on query param state", () => {
    mockRouter.setCurrentUrl("/some-url");
    mockRouter.query = {
      member: "1",
      action: "createTeam",
      startDate: "2022-01-01",
      endDate: "2022-01-02",
    };
    render(
      <AuditLog
        team={{
          id: 1,
          name: "Team 1",
          slug: "team-1",
          creator: 1,
          suspended: false,
        }}
      />,
    );
    expect(useTeamAuditLog).toHaveBeenCalledTimes(1);
    expect(useTeamAuditLog).toHaveBeenCalledWith(1, {
      from: parse("2022-01-01", DATE_FORMAT, new Date()).getTime(),
      to: endOfDay(parse("2022-01-02", DATE_FORMAT, new Date())).getTime(),
      memberId: "1",
      action: "createTeam",
    });
  });

  it("cannot load the next page if there are no more entries", () => {
    useTeamAuditLog.mockReturnValue({
      entries: [],
      isLoading: false,
      loadNextPage,
      hasMore: false,
    });
    render(
      <AuditLog
        team={{
          id: 1,
          name: "Team 1",
          slug: "team-1",
          creator: 1,
          suspended: false,
        }}
      />,
    );
    expect(loadNextPage).not.toHaveBeenCalled();
  });

  it("loads the next page when the users clicks loadNextPage", async () => {
    useTeamAuditLog.mockReturnValue({
      entries: [],
      isLoading: false,
      loadNextPage,
      hasMore: true,
    });
    render(
      <AuditLog
        team={{
          id: 1,
          name: "Team 1",
          slug: "team-1",
          creator: 1,
          suspended: false,
        }}
      />,
    );
    expect(loadNextPage).not.toHaveBeenCalled();

    const loadMore = screen.getByText("Load more");
    expect(loadMore).not.toBeDisabled();
    await act(() => {
      // Click the load more button
      loadMore.click();
    });
    expect(loadNextPage).toHaveBeenCalledTimes(1);
  });
});
