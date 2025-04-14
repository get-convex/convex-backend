import type {
  MemberResponse,
  ProjectDetails,
  ProjectMemberRoleResponse,
  UpdateProjectRolesArgs,
  Team,
  TeamMemberResponse,
} from "generatedApi";
import { useRouter } from "next/router";
import { useRef, useState } from "react";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Combobox, Option } from "@ui/Combobox";
import { Spinner } from "@ui/Spinner";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { CaretSortIcon } from "@radix-ui/react-icons";
import { useMount } from "react-use";
import classNames from "classnames";
import startCase from "lodash/startCase";
import Link from "next/link";
import { MemberProjectRolesModal } from "./MemberProjectRolesModal";

export const roleOptions: Option<"admin" | "developer">[] = [
  { label: "Admin", value: "admin" },
  { label: "Developer", value: "developer" },
];

type TeamMemberListItemProps = {
  team: Team;
  myProfile: MemberResponse;
  member: TeamMemberResponse;
  members: TeamMemberResponse[];
  canChangeRole: boolean;
  onChangeRole: (body: {
    memberId: number;
    role: "admin" | "developer";
  }) => Promise<Response>;
  onRemoveMember: (body: { memberId: number }) => Promise<Response>;
  onUpdateProjectRoles: (body: UpdateProjectRolesArgs) => Promise<undefined>;
  hasAdminPermissions: boolean;
  projectRoles: ProjectMemberRoleResponse[];
  projects: ProjectDetails[];
};
export function TeamMemberListItem({
  team,
  myProfile,
  member,
  members,
  canChangeRole,
  onChangeRole,
  onUpdateProjectRoles,
  onRemoveMember,
  hasAdminPermissions,
  projectRoles,
  projects,
}: TeamMemberListItemProps) {
  const router = useRouter();
  const isMemberTheLastAdmin =
    members.filter((m) => m.role === "admin" && m.id !== member.id).length ===
    0;
  const isMemberMe = member.id === myProfile?.id;

  const canManageMember =
    (hasAdminPermissions || isMemberMe) && !isMemberTheLastAdmin;

  const isHighlighted = window.location.hash === `#${member.id}`;

  const ref = useRef<HTMLDivElement | null>(null);
  useMount(() => {
    isHighlighted && ref.current?.scrollIntoView();
  });

  let removeMemberMessage = "";
  if (isMemberTheLastAdmin) {
    removeMemberMessage =
      "You cannot remove the last admin from this team. Contact us for help at support@convex.dev";
  } else if (!canManageMember) {
    removeMemberMessage =
      "You do not have permission to remove members from this team.";
  }

  let updateRoleMessage = "";
  if (isMemberTheLastAdmin) {
    updateRoleMessage = "You cannot change the role of the last admin.";
  } else if (!hasAdminPermissions) {
    updateRoleMessage = "You do not have permission to change member roles.";
  }

  const [showRemoveMember, setShowRemoveMember] = useState(false);

  const [isUpdatingRole, setIsUpdatingRole] = useState(false);

  const confirmationDisplayName = member.name
    ? `${member.name} (${member.email})`
    : member.email;

  const [showProjecRolesModal, setShowProjectRolesModal] = useState(false);

  return (
    <div
      ref={ref}
      className={classNames(
        "flex flex-wrap justify-between items-center gap-4 py-2",
        isHighlighted
          ? "bg-highlight px-2 -mx-2 rounded border"
          : "border-b last:border-b-0",
      )}
    >
      <div className="flex max-w-[40%] flex-col sm:max-w-[50%] md:max-w-[80%]">
        {member.name && (
          <div className="text-sm text-content-primary">{member.name}</div>
        )}
        <div
          className={`${
            member.name
              ? "text-xs text-content-secondary"
              : "text-sm text-content-primary"
          }`}
        >
          {member.email}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-2">
          {!canChangeRole ? (
            <div className="text-sm text-content-primary">
              {startCase(member.role)}
            </div>
          ) : !canManageMember ? (
            // Combobox is difficult to create a disabled state for, so we're using a div here that looks like a disabled input
            <Tooltip tip={updateRoleMessage}>
              <div className="flex cursor-not-allowed items-center gap-1 rounded border bg-background-tertiary px-3 py-2 text-content-secondary">
                {startCase(member.role)}
                <CaretSortIcon className="h-5 w-5" />
              </div>
            </Tooltip>
          ) : (
            <>
              {isUpdatingRole && <Spinner />}
              <Combobox
                buttonClasses="w-fit"
                disableSearch
                label="Role"
                options={roleOptions}
                selectedOption={member.role}
                buttonProps={{
                  tip: (
                    <span>
                      Change this member's{" "}
                      <Link
                        href="https://docs.convex.dev/dashboard/teams#roles-and-permissions"
                        className="underline"
                      >
                        team role
                      </Link>
                      .
                    </span>
                  ),
                  tipSide: "top",
                }}
                setSelectedOption={async (role) => {
                  if (!role) {
                    return;
                  }
                  setIsUpdatingRole(true);
                  try {
                    await onChangeRole({ memberId: member.id, role });
                  } finally {
                    setIsUpdatingRole(false);
                  }
                }}
              />
            </>
          )}
        </div>
        <Button
          variant="neutral"
          onClick={() => setShowProjectRolesModal(true)}
        >
          Project Roles ({projectRoles?.length || 0})
        </Button>
        <Button
          variant="danger"
          disabled={!canManageMember}
          tip={removeMemberMessage}
          onClick={() => setShowRemoveMember(true)}
        >
          {isMemberMe ? "Leave team" : "Remove member"}
        </Button>
        {showRemoveMember && (
          <ConfirmationDialog
            onClose={() => setShowRemoveMember(false)}
            onConfirm={async () => {
              await onRemoveMember({ memberId: member.id });
              if (isMemberMe) {
                await router.push("/");
              }
            }}
            dialogTitle={isMemberMe ? "Leave team" : "Remove team member"}
            dialogBody={
              isMemberMe
                ? `You are about to leave ${team.name}, are you sure you want to continue?`
                : `You are about to remove ${confirmationDisplayName} from ${team.name}, are you sure you want to continue?`
            }
            confirmText="Confirm"
          />
        )}
        {showProjecRolesModal && (
          <MemberProjectRolesModal
            member={member}
            team={team}
            projects={projects}
            projectRoles={projectRoles}
            onClose={() => setShowProjectRolesModal(false)}
            onUpdateProjectRoles={onUpdateProjectRoles}
          />
        )}
      </div>
    </div>
  );
}
