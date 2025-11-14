import { Sheet } from "@ui/Sheet";
import {
  AuditLogEventResponse,
  MemberResponse,
  ProjectDetails,
  TeamResponse,
} from "generatedApi";
import { AuditLogItem } from "./AuditLogItem";

export function AuditLogContent({
  team,
  projects,
  members,
  entries,
}: {
  team: TeamResponse;
  projects: ProjectDetails[];
  members: MemberResponse[];
  entries: AuditLogEventResponse[];
}) {
  return (
    <Sheet
      // Account for the load more button below the content
      className="flex-col overflow-y-auto py-4"
      padding={false}
    >
      {entries.length === 0 ? (
        <NoEntries />
      ) : (
        entries.map((entry) => (
          <AuditLogItem
            team={team}
            projects={projects}
            entry={entry}
            key={entry.createTime}
            memberId={entryMemberId(entry)}
            members={members}
          />
        ))
      )}
    </Sheet>
  );
}

function entryMemberId(entry: AuditLogEventResponse) {
  return typeof entry.actor === "object"
    ? "member" in entry.actor
      ? entry.actor.member.member_id
      : "serviceAccount" in entry.actor
        ? entry.actor.serviceAccount.member_id
        : null
    : null;
}

function NoEntries() {
  return (
    <div
      className="flex h-full flex-1 flex-col items-center justify-center"
      data-testid="no-entries"
    >
      <div className="mx-2 flex flex-col items-center gap-2 text-content-secondary">
        No audit log entries matching the selected date range and filters.
      </div>
    </div>
  );
}
