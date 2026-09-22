import { useEffect, useState } from "react";
import { ArrowRightIcon, InfoCircledIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { Loading } from "@ui/Loading";
import { Modal } from "@ui/Modal";
import { Tooltip } from "@ui/Tooltip";
import { cn } from "@ui/cn";
import type {
  StagedDirectoryMemberResponse,
  TeamMemberCustomRole,
  TeamResponse,
} from "generatedApi";
import {
  STAGED_MEMBERS_PAGE_SIZE,
  useEnableDirectorySync,
  useStagedDirectoryMembers,
} from "api/directorySync";
import { PaginationControls } from "elements/PaginationControls";
import {
  useCursorPagination,
  useSnapBackOnEmptyPage,
} from "hooks/useCursorPagination";
import { RoleDisplay } from "../RoleDisplay";
import { TABLE_CELL, TABLE_HEADER_CELL } from "./SettingsSheet";
import { StatusDot } from "./StatusBadge";

export const SYNC_DELAY_NOTE =
  "Directory changes can take up to an hour to sync, so this list may lag behind your identity provider.";

export type StagedStatus =
  | "inDirectory"
  | "suspended"
  | "notInDirectory"
  | "canJoin";

function sameRoleIds(
  a: TeamMemberCustomRole[] | null | undefined,
  b: TeamMemberCustomRole[] | null | undefined,
) {
  const aIds = (a ?? []).map((r) => r.id).sort((x, y) => x - y);
  const bIds = (b ?? []).map((r) => r.id).sort((x, y) => x - y);
  return aIds.length === bIds.length && aIds.every((id, i) => id === bIds[i]);
}

/** What enabling directory sync does to one staged row. */
export function classifyStagedItem(item: StagedDirectoryMemberResponse): {
  status: StagedStatus;
  roleChanges: boolean;
} {
  const member = item.member ?? undefined;
  const directoryUser = item.directoryUser ?? undefined;
  if (!directoryUser) {
    return { status: "notInDirectory", roleChanges: false };
  }
  // Decided before the roster split below: a user the directory doesn't hold
  // active confers nothing either way — a member linked to one is taken off
  // the team, and a non-member is not offered it at all.
  if (directoryUser.state !== "active") {
    return { status: "suspended", roleChanges: false };
  }
  if (!member) {
    return { status: "canJoin", roleChanges: false };
  }
  return {
    status: "inDirectory",
    roleChanges:
      member.role !== directoryUser.role ||
      !sameRoleIds(member.customRoles, directoryUser.customRoles),
  };
}

export function ReviewDirectoryChangesModal({
  team,
  enabled,
  onClose,
}: {
  team: TeamResponse;
  /** Once management is on the roster only lists who can still join. */
  enabled: boolean;
  onClose: () => void;
}) {
  const pagination = useCursorPagination();
  const { data, isLoading, error } = useStagedDirectoryMembers(
    team.id,
    pagination.currentCursor,
  );
  const items = data?.items;
  useSnapBackOnEmptyPage(pagination, { isLoading, currentPageItems: items });
  const hasMore = data?.pagination.hasMore ?? false;
  const showPagination = hasMore || pagination.currentPage > 1;

  const enableDirectorySync = useEnableDirectorySync(team.id);
  const [isEnabling, setIsEnabling] = useState(false);
  const [enableError, setEnableError] = useState<string>();
  const [accepted, setAccepted] = useState(false);
  // The acknowledgement covers every staged change, but only a page of them is
  // ever on screen. Cursor pagination has no jump-to-last, so reaching the
  // final page means having stepped through all of them. Sticky, so paging
  // back to re-read a row doesn't retract the acknowledgement.
  const [reachedLastPage, setReachedLastPage] = useState(false);
  useEffect(() => {
    if (items !== undefined && !hasMore) {
      setReachedLastPage(true);
    }
  }, [items, hasMore]);

  const classified = (items ?? []).map((item) => ({
    item,
    ...classifyStagedItem(item),
  }));
  return (
    <Modal
      size="lg"
      title={enabled ? "Pending members" : "Review and enable directory sync"}
      description={
        enabled
          ? "Directory users who have not joined the team yet."
          : "Once enabled, your directory sets the roles of the members it covers and removes the ones it suspends. Review the changes below before enabling."
      }
      onBeforeClose={() => !isEnabling}
      onClose={onClose}
    >
      <div className="mt-3 flex flex-col gap-4">
        {error ? (
          <Callout variant="error" className="mt-0">
            {error.code === "DirectoryNotConfigured"
              ? "The directory for this team has not been synced yet. Please try again later or check your configuration."
              : (error.message ?? "Failed to load the staged members.")}
          </Callout>
        ) : items === undefined ? (
          <Loading fullHeight={false} className="h-24" />
        ) : items.length === 0 ? (
          // Bordered like the table it stands in for, so the panel reads as
          // an empty list rather than a gap under the heading.
          <p className="rounded-lg border px-6 py-8 text-center text-sm text-content-secondary">
            {enabled
              ? "No pending members. Members your directory provisions can join this team themselves by selecting it in the Convex Dashboard."
              : "No members or directory users to review yet."}
          </p>
        ) : (
          <div className="scrollbar max-h-[50vh] overflow-auto rounded-lg border">
            <table className="w-full border-collapse">
              <thead>
                <tr className="border-b">
                  <th className={TABLE_HEADER_CELL}>Member</th>
                  {/* The groups take the slack, so the roles beside them read
                      as one line. */}
                  <th className={cn(TABLE_HEADER_CELL, "w-full")}>
                    Directory groups
                  </th>
                  {/* Right aligned so it reads next to the role it becomes,
                      across however much room the groups take. */}
                  {/* The roles keep a column's worth of room whatever the
                      groups take, so a custom role's chips have somewhere to
                      sit and neither header wraps. */}
                  {!enabled && (
                    <>
                      <th
                        className={cn(
                          TABLE_HEADER_CELL,
                          "min-w-56 text-right whitespace-nowrap",
                        )}
                      >
                        Current role
                      </th>
                      <th className={cn(TABLE_HEADER_CELL, "w-0 px-0")}>
                        <span className="sr-only">Becomes</span>
                      </th>
                    </>
                  )}
                  <th
                    className={cn(
                      TABLE_HEADER_CELL,
                      "min-w-56 whitespace-nowrap",
                    )}
                  >
                    Directory role
                  </th>
                </tr>
              </thead>
              <tbody>
                {classified.map(({ item, status }) => {
                  const member = item.member ?? undefined;
                  const directoryUser = item.directoryUser ?? undefined;
                  return (
                    <tr
                      key={
                        member
                          ? `member-${member.id}`
                          : `user-${directoryUser?.directoryUserId}`
                      }
                      className="border-b last:border-b-0"
                    >
                      <td className={TABLE_CELL}>
                        <div className="flex flex-col">
                          <span className="text-sm">
                            {member?.name ||
                              member?.email ||
                              directoryUser?.email}
                          </span>
                          {member?.name && (
                            <span className="text-xs text-content-secondary">
                              {member.email}
                            </span>
                          )}
                          {status === "suspended" && (
                            <span className="mt-1">
                              <StatusDot
                                label="Suspended in directory"
                                tone="error"
                              />
                            </span>
                          )}
                        </div>
                      </td>
                      <td className={TABLE_CELL}>
                        {!directoryUser ? (
                          <span className="flex items-center gap-1 text-sm text-content-secondary">
                            Not in directory
                            <Tooltip
                              side="right"
                              aria-label="About team members who are not in the directory"
                              tip="This team member will continue to have access to Convex until they are removed from the team or added to the directory."
                            >
                              <InfoCircledIcon className="text-content-tertiary" />
                            </Tooltip>
                          </span>
                        ) : directoryUser.groups.length > 0 ? (
                          // Plain text: these name where the role comes from,
                          // and chips would read as something to click.
                          <span className="text-sm text-content-primary">
                            {directoryUser.groups
                              .map((group) => group.name)
                              .join(", ")}
                          </span>
                        ) : (
                          <span className="text-sm text-content-secondary">
                            —
                          </span>
                        )}
                      </td>
                      {!enabled && (
                        <>
                          <td className={cn(TABLE_CELL, "whitespace-nowrap")}>
                            {member ? (
                              <RoleDisplay
                                role={member.role}
                                customRoles={member.customRoles}
                                teamSlug={team.slug}
                                align="right"
                              />
                            ) : (
                              <div className="flex items-center justify-end gap-1 text-sm text-content-secondary">
                                Not in team
                                <Tooltip
                                  side="left"
                                  aria-label="About directory users who are not on the team"
                                  tip={
                                    status === "suspended"
                                      ? "This user is not active in your directory, so they will not be offered the team."
                                      : "The Convex member who owns this email address will be offered to join the team once directory sync is enabled."
                                  }
                                >
                                  <InfoCircledIcon className="text-content-tertiary" />
                                </Tooltip>
                              </div>
                            )}
                          </td>
                          {/* Every row reads as one role becoming another,
                              whether or not the two differ. */}
                          <td className={cn(TABLE_CELL, "w-0 px-0")}>
                            <ArrowRightIcon className="text-content-secondary" />
                          </td>
                        </>
                      )}
                      <td className={cn(TABLE_CELL, "whitespace-nowrap")}>
                        {status === "suspended" ? (
                          <span className="text-sm text-content-secondary">
                            {member ? "Removed from team" : "Cannot join"}
                          </span>
                        ) : directoryUser ? (
                          <RoleDisplay
                            role={directoryUser.role}
                            customRoles={directoryUser.customRoles}
                            teamSlug={team.slug}
                          />
                        ) : member ? (
                          // The directory does not cover them, so the role
                          // they hold is the role they keep.
                          <div className="flex items-center gap-1">
                            <RoleDisplay
                              role={member.role}
                              customRoles={member.customRoles}
                              teamSlug={team.slug}
                            />
                            <Tooltip
                              side="left"
                              aria-label="About the role of a member who is not in the directory"
                              tip="This team member is not listed in the directory, so they keep the role they have today."
                            >
                              <InfoCircledIcon className="text-content-tertiary" />
                            </Tooltip>
                          </div>
                        ) : null}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}

        <div className="flex items-center justify-between gap-4">
          <p className="text-xs text-content-secondary">{SYNC_DELAY_NOTE}</p>
          {showPagination && (
            <PaginationControls
              isCursorBasedPagination
              currentPage={pagination.currentPage}
              hasMore={hasMore}
              pageSize={STAGED_MEMBERS_PAGE_SIZE}
              onPageSizeChange={() => {}}
              // The endpoint fixes the page size, so there is nothing to choose.
              showPageSize={false}
              className="justify-end"
              onPreviousPage={pagination.onPreviousPage}
              onNextPage={() =>
                pagination.onNextPage(data?.pagination.nextCursor)
              }
              canGoPrevious={pagination.canGoPrevious}
            />
          )}
        </div>

        {enableError && (
          <Callout variant="error" className="mt-0">
            {enableError}
          </Callout>
        )}

        {enabled ? (
          <div className="flex justify-end">
            <Button variant="neutral" onClick={onClose}>
              Close
            </Button>
          </div>
        ) : (
          <div className="flex flex-col items-start gap-3">
            <Tooltip
              asChild
              side="right"
              tip={
                reachedLastPage
                  ? undefined
                  : "Please review all pages of staged role changes before enabling directory sync. This checkbox will become available once you visit the last page of staged changes."
              }
            >
              <label
                className={cn(
                  "flex items-center gap-2 text-sm text-content-primary",
                  // The label is the tooltip's trigger, so the cursor has to
                  // say "disabled" across the copy, not just over the box.
                  reachedLastPage ? "cursor-pointer" : "cursor-not-allowed",
                )}
              >
                <Checkbox
                  className="ml-px"
                  checked={accepted}
                  onChange={() => setAccepted(!accepted)}
                  disabled={isEnabling || !reachedLastPage}
                />
                I understand and accept the role changes that will occur after
                enabling directory sync.
              </label>
            </Tooltip>
            <Button
              loading={isEnabling}
              disabled={!accepted || items === undefined || error !== undefined}
              onClick={async () => {
                setIsEnabling(true);
                setEnableError(undefined);
                try {
                  await enableDirectorySync();
                  onClose();
                } catch (e: any) {
                  setEnableError(e.message);
                } finally {
                  setIsEnabling(false);
                }
              }}
            >
              Enable directory sync
            </Button>
          </div>
        )}
      </div>
    </Modal>
  );
}
