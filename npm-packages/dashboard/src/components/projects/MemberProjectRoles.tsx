import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Sheet } from "@ui/Sheet";
import { LoadingTransition } from "@ui/Loading";
import { useCurrentProject } from "api/projects";
import { useCurrentTeam, useTeamMembers } from "api/teams";
import {
  useHasProjectAdminPermissions,
  useUpdateProjectRoles,
  useCurrentProjectRoles,
} from "api/roles";
import { TeamMemberResponse } from "generatedApi";
import Link from "next/link";
import { useState } from "react";
import sortBy from "lodash/sortBy";
import { ProjectAdminFormModal } from "./ProjectAdminsFormModal";

export function MemberProjectRoles() {
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const projectRoles = useCurrentProjectRoles();
  const members = useTeamMembers(team?.id);
  const membersWithProjectAccess = members
    ?.map((member) => {
      const memberRole = projectRoles?.find(
        (role) => role.memberId === member.id,
      );
      return {
        ...member,
        isProjectAdmin: !!memberRole,
      };
    })
    .filter(
      (member): member is TeamMemberResponse & { isProjectAdmin: boolean } =>
        member.isProjectAdmin || member.role === "admin",
    );

  const [showProjectAdminsModal, setShowProjectAdminsModal] = useState(false);

  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);

  const updateProjectRoles = useUpdateProjectRoles(team?.id);

  return (
    <Sheet className="flex flex-col gap-2 text-sm">
      <div className="flex justify-between">
        <h3>Project Admins</h3>{" "}
        <Button
          className="ml-auto w-fit"
          disabled={!hasAdminPermissions}
          tip={
            !hasAdminPermissions &&
            "You do not have permission to manage project admins."
          }
          onClick={() => setShowProjectAdminsModal(true)}
        >
          Manage Project Admins
        </Button>
      </div>
      <p>These team members have administrative access to this project. </p>
      <p className="max-w-prose">
        All other team members may create dev and preview deployments, and have
        read-only access to production data.{" "}
        <Link
          href="https://docs.convex.dev/dashboard/teams#roles-and-permissions"
          target="_blank"
          className="text-content-link hover:underline"
        >
          Learn more
        </Link>{" "}
        about project permissions.
      </p>
      <LoadingTransition>
        {membersWithProjectAccess && (
          <div className="flex w-full flex-col">
            {sortBy(membersWithProjectAccess, (member) =>
              (member.name || member.email).toLocaleLowerCase(),
            ).map((member, idx) => (
              <div
                className="flex items-center justify-between border-b py-2 last:border-b-0"
                key={idx}
              >
                <div className="flex flex-col truncate">
                  {member.name && (
                    <div className="text-sm text-content-primary">
                      {member.name}
                    </div>
                  )}
                  <div
                    className={`truncate ${
                      member.name
                        ? "text-xs text-content-secondary"
                        : "text-sm text-content-primary"
                    }`}
                  >
                    {member.email}
                  </div>
                </div>
                <div className="flex gap-1">
                  {member.role === "admin" && (
                    <Tooltip
                      tip={
                        <div className="flex flex-col">
                          <p>
                            This member can manage all projects in the team
                            because they are a team admin.
                          </p>{" "}
                          <p>
                            You may view and manage team admins on the{" "}
                            <Link
                              className="underline"
                              href={`/t/${team?.slug}/settings/members`}
                              passHref
                            >
                              member settings
                            </Link>{" "}
                            page.
                          </p>
                        </div>
                      }
                    >
                      <div className="rounded border p-1 text-xs">
                        Team Admin
                      </div>
                    </Tooltip>
                  )}
                  {member.isProjectAdmin && (
                    <Tooltip tip="This member can manage this project because they are a project admin.">
                      <div className="rounded border p-1 text-xs">
                        Project Admin
                      </div>
                    </Tooltip>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </LoadingTransition>
      {showProjectAdminsModal &&
        team &&
        members &&
        project &&
        projectRoles !== undefined && (
          <ProjectAdminFormModal
            project={project}
            members={members}
            projectRoles={projectRoles}
            onUpdateProjectRoles={async (args) => {
              await updateProjectRoles(args);
            }}
            onClose={() => setShowProjectAdminsModal(false)}
          />
        )}
    </Sheet>
  );
}
