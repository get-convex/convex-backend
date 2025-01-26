import { LoadingTransition, Button, useDateFilters } from "dashboard-common";
import { endOfDay } from "date-fns";
import { useTeamAuditLog } from "hooks/api";
import { useProjects } from "api/projects";
import { useTeamMembers } from "api/teams";
import { AuditLogAction, Team } from "generatedApi";
import { useRouter } from "next/router";
import { AuditLogContent } from "./AuditLogContent";
import { AuditLogToolbar } from "./AuditLogToolbar";

export function AuditLog({ team }: { team: Team }) {
  const projects = useProjects(team.id);
  const members = useTeamMembers(team.id);

  const router = useRouter();

  // Filter state management
  const { startDate, endDate, setDate } = useDateFilters(router);

  const { member, action } = router.query;
  const selectedMember = Array.isArray(member) ? member[0] : member || null;
  const selectedAction = Array.isArray(action)
    ? (action[0] as AuditLogAction)
    : (action as AuditLogAction) || null;

  const setSelectedMember = (m: string) => {
    void router.push(
      {
        query: { ...router.query, member: m === "all_members" ? undefined : m },
      },
      undefined,
      { shallow: true },
    );
  };
  const setSelectedAction = (a: AuditLogAction | "all_actions") => {
    void router.push(
      {
        query: { ...router.query, action: a === "all_actions" ? undefined : a },
      },
      undefined,
      { shallow: true },
    );
  };

  // Load the data for the audit log
  const { entries, isLoading, loadNextPage, hasMore } = useTeamAuditLog(
    team.id,
    {
      from: startDate.getTime(),
      to: endOfDay(endDate).getTime(),
      memberId: selectedMember === "all_members" ? null : selectedMember,
      action: selectedAction,
    },
  );

  return (
    <>
      <h2>Audit Log</h2>
      <div className="flex grow flex-col gap-4 overflow-y-hidden">
        {members && (
          <AuditLogToolbar
            selectedMember={
              selectedMember === null ? "all_members" : selectedMember
            }
            setSelectedMember={setSelectedMember}
            selectedAction={
              selectedAction === null ? "all_actions" : selectedAction
            }
            setSelectedAction={setSelectedAction}
            selectedStartDay={startDate}
            selectedEndDay={endDate}
            setDate={setDate}
            members={members}
          />
        )}
        <LoadingTransition>
          {projects && members && !isLoading && entries !== undefined ? (
            <div className="flex w-full flex-col gap-4 overflow-y-auto">
              <AuditLogContent
                {...{
                  team,
                  projects,
                  members,
                  entries,
                }}
              />
              <Button
                onClick={loadNextPage}
                className="ml-auto w-fit"
                variant="neutral"
                disabled={!hasMore}
                tip={hasMore ? undefined : "There are no more entries to load."}
              >
                Load more
              </Button>
            </div>
          ) : undefined}
        </LoadingTransition>
      </div>
    </>
  );
}
