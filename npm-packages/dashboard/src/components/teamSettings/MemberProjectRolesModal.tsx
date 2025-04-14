import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Spinner } from "@ui/Spinner";
import { Checkbox } from "@ui/Checkbox";
import { Modal } from "@ui/Modal";
import difference from "lodash/difference";
import React, { useState } from "react";
import type {
  Team,
  ProjectMemberRoleResponse,
  ProjectDetails,
  UpdateProjectRolesArgs,
  TeamMemberResponse,
} from "generatedApi";
import Link from "next/link";
import { useHasProjectAdminPermissions } from "api/roles";
import sortBy from "lodash/sortBy";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { ProjectLink } from "./AuditLogItem";

export function MemberProjectRolesModal({
  team,
  projects,
  member,
  projectRoles,
  onUpdateProjectRoles,
  onClose,
}: {
  team: Team;
  projects: ProjectDetails[];
  member: TeamMemberResponse;
  projectRoles: ProjectMemberRoleResponse[];
  onUpdateProjectRoles: (body: UpdateProjectRolesArgs) => Promise<undefined>;
  onClose: () => void;
}) {
  const originalProjectRoles = projectRoles.map(
    (projectRole) => projectRole.projectId,
  );
  const [newProjectRoles, setNewProjectRoles] = useState(originalProjectRoles);

  const addedProjects = difference(newProjectRoles, originalProjectRoles);
  const removedProjects = difference(originalProjectRoles, newProjectRoles);

  const [isSaving, setIsSaving] = useState(false);

  const closeWithConfirmation = () => {
    if (addedProjects.length > 0 || removedProjects.length > 0) {
      // eslint-disable-next-line no-alert
      const shouldClose = window.confirm(
        "Closing the popup will clear your unsaved changes. Are you sure you want to continue?",
      );
      if (!shouldClose) {
        return;
      }
    }
    onClose();
  };
  return (
    <Modal
      title="Manage Project Roles"
      size="md"
      description={
        <div className="flex flex-col gap-2 text-sm">
          <p>
            Manage Project Admin access for{" "}
            <TeamMemberLink
              memberId={member.id}
              name={member.name || member.email}
            />
            .
          </p>
          <p>
            Project Admins have administrative access to a project, including
            the ability to delete the project and write to production.
          </p>
        </div>
      }
      onClose={closeWithConfirmation}
    >
      <form
        className="flex w-full flex-col gap-2"
        onSubmit={async (e) => {
          e.preventDefault();
          setIsSaving(true);
          try {
            await onUpdateProjectRoles({
              updates: [
                ...addedProjects.map((added) => ({
                  memberId: member.id,
                  projectId: added,
                  role: "admin" as const,
                })),
                ...removedProjects.map((removed) => ({
                  memberId: member.id,
                  projectId: removed,
                })),
              ],
            });
            onClose();
          } finally {
            setIsSaving(false);
          }
        }}
      >
        <div className="max-h-[60vh] overflow-auto scrollbar">
          {sortBy(projects, (project) => project.name.toLocaleLowerCase()).map(
            (project) => (
              <ProjectRoleItem
                key={project.id}
                project={project}
                projects={projects}
                team={team}
                originalProjectRoles={originalProjectRoles}
                newProjectRoles={newProjectRoles}
                setNewProjectRoles={setNewProjectRoles}
              />
            ),
          )}
        </div>
        <p className="text-xs text-content-secondary">
          Pro-tip! You can manage the Project Admin role for multiple members at
          the same time on the{" "}
          <Link
            href="https://docs.convex.dev/dashboard/projects#project-settings"
            className="text-content-link hover:underline"
          >
            Project Settings
          </Link>{" "}
          page.{" "}
        </p>
        <div className="ml-auto flex items-center gap-2 text-right">
          <div className="text-xs">
            {addedProjects.length > 0 && (
              <div className="text-content-success">
                Add {addedProjects.length} role
                {addedProjects.length > 1 ? "s" : ""}
              </div>
            )}
            {removedProjects.length > 0 && (
              <div className="text-content-error">
                Remove {removedProjects.length} role
                {removedProjects.length > 1 ? "s" : ""}
              </div>
            )}
          </div>

          <Button
            type="submit"
            disabled={
              (addedProjects.length === 0 && removedProjects.length === 0) ||
              isSaving
            }
            icon={isSaving && <Spinner />}
          >
            Save
          </Button>
        </div>
      </form>
    </Modal>
  );
}

function ProjectRoleItem({
  project,
  projects,
  team,
  originalProjectRoles,
  newProjectRoles,
  setNewProjectRoles,
}: {
  project: ProjectDetails;
  projects: ProjectDetails[];
  team: Team;
  originalProjectRoles: number[];
  newProjectRoles: number[];
  setNewProjectRoles: React.Dispatch<React.SetStateAction<number[]>>;
}) {
  const hasAdminPermissions = useHasProjectAdminPermissions(project.id);
  return (
    <div className="flex h-12 items-center gap-4 border-b px-1 py-2 last:border-b-0">
      <Tooltip
        tip={
          !hasAdminPermissions &&
          `You do not have permission to manage roles for ${project.name}`
        }
        side="left"
      >
        <Checkbox
          checked={newProjectRoles.includes(project.id)}
          disabled={!hasAdminPermissions}
          onChange={() => {
            setNewProjectRoles((prev) =>
              newProjectRoles.includes(project.id)
                ? prev.filter((id) => id !== project.id)
                : [...prev, project.id],
            );
          }}
        />
      </Tooltip>
      <ProjectLink
        metadata={{}}
        projects={projects}
        projectId={project.id}
        team={team}
      />
      <div className="ml-auto rounded p-1 text-xs">
        {originalProjectRoles.includes(project.id) &&
          !newProjectRoles.includes(project.id) && (
            <div className="rounded bg-background-error p-1 text-xs text-content-error">
              Role will be removed
            </div>
          )}
        {!originalProjectRoles.includes(project.id) &&
          newProjectRoles.includes(project.id) && (
            <div className="rounded bg-background-success p-1 text-xs text-content-success">
              Role will be added
            </div>
          )}
      </div>
    </div>
  );
}
