import { Command } from "cmdk";
import { TrashIcon } from "@radix-ui/react-icons";
import {
  useCurrentProject,
  useDeleteProjects,
  useProjects,
} from "api/projects";
import { useCurrentTeam } from "api/teams";
import React from "react";
import { Checkbox } from "@ui/Checkbox";
import { useHotkeys } from "react-hotkeys-hook";
import { buttonClasses } from "@ui/Button";
import { Spinner } from "@ui/Spinner";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { Tooltip } from "@ui/Tooltip";
import { useClickAway } from "react-use";
import { cn } from "@ui/cn";
import { useRouter } from "next/router";

export function CommandPalette() {
  const [open, setOpen] = React.useState(false);
  const [search, setSearch] = React.useState("");
  const [pages, setPages] = React.useState<string[]>([]);
  const page = pages[pages.length - 1];

  useHotkeys(["meta+k", "ctrl+k"], (event) => {
    event.preventDefault();
    setOpen((isOpen) => !isOpen);
  });

  useHotkeys(["escape", "backspace"], (event) => {
    if (
      pages.length > 0 &&
      (event.key === "Escape" || (event.key === "Backspace" && !search))
    ) {
      setPages((currentPages) => currentPages.slice(0, -1));
    } else if (event.key === "Escape") {
      setOpen(false);
    }
  });

  const ref = React.useRef<HTMLDivElement>(null);

  useClickAway(ref, () => {
    setOpen(false);
  });

  const isTeamAdmin = true;
  const { commandPalette, commandPaletteDeleteProjects } = useLaunchDarkly();

  if (!commandPalette) {
    return null;
  }

  return (
    <Command.Dialog
      open={open}
      ref={ref}
      label="Convex Command Palette"
      title="Convex Command Palette"
    >
      <Command.Input
        placeholder={
          page === "delete-projects"
            ? "Search projects..."
            : "What do you want to do?"
        }
        value={search}
        onValueChange={setSearch}
      />
      <Command.List>
        {!page && (
          <Command.Group heading="Projects">
            {commandPaletteDeleteProjects && (
              <Tooltip
                side="right"
                tip={
                  !isTeamAdmin
                    ? "You must be a team admin to delete projects in bulk."
                    : undefined
                }
              >
                <Command.Item
                  onSelect={() =>
                    setPages((currentPages) => [
                      ...currentPages,
                      "delete-projects",
                    ])
                  }
                  disabled={!isTeamAdmin}
                >
                  <TrashIcon className="size-4" />
                  Delete Projects
                </Command.Item>
              </Tooltip>
            )}
          </Command.Group>
        )}

        {page === "delete-projects" && (
          <DeleteProjectsPage onClose={() => setOpen(false)} />
        )}
        <Command.Empty>No results found.</Command.Empty>
      </Command.List>
    </Command.Dialog>
  );
}

function DeleteProjectsPage({ onClose }: { onClose: () => void }) {
  const router = useRouter();
  const [projectIds, setProjectIds] = React.useState<number[]>([]);
  const [lastSelectedIndex, setLastSelectedIndex] = React.useState<
    number | null
  >(null);

  const currentTeam = useCurrentTeam();
  const currentProject = useCurrentProject();
  const projects = useProjects(currentTeam?.id);
  const deleteProjects = useDeleteProjects(currentTeam?.id);
  const [isSubmitting, setIsSubmitting] = React.useState(false);

  const handleDeleteProjects = async () => {
    if (projectIds.length === 0) {
      return;
    }

    setIsSubmitting(true);
    setProjectIds([]);
    try {
      if (currentProject && projectIds.includes(currentProject.id)) {
        await router.push(`/t/${currentTeam?.slug}`);
      }
      await deleteProjects({ projectIds });
      onClose();
    } finally {
      setTimeout(() => {
        setIsSubmitting(false);
      }, 0);
    }
  };

  const toggleProject = (
    projectId: number,
    index: number,
    event: React.MouseEvent,
  ) => {
    if (event.nativeEvent?.shiftKey && lastSelectedIndex !== null) {
      // Implement shift+click selection
      const start = Math.min(lastSelectedIndex, index);
      const end = Math.max(lastSelectedIndex, index);
      const isSelected = projectIds.includes(projectId);
      const newProjectIds = new Set(projectIds);

      if (isSelected) {
        // Unselect this row and all the next consecutive selected
        for (let i = start; i <= end; i++) {
          const id = projects?.[i]?.id;
          if (id && projectIds.includes(id)) {
            newProjectIds.delete(id);
          }
        }
      } else {
        // If there are no rows selected above, first try to select from below
        const firstSelected =
          projects?.findIndex((p) => projectIds.includes(p.id)) ?? -1;
        if (firstSelected > index) {
          for (let i = index; i < firstSelected; i++) {
            const id = projects?.[i]?.id;
            if (id) {
              newProjectIds.add(id);
            }
          }
        } else {
          // Select all rows from the first unselected row above
          for (let i = index; i >= 0; i--) {
            const id = projects?.[i]?.id;
            if (id && !projectIds.includes(id)) {
              newProjectIds.add(id);
            } else {
              break;
            }
          }
        }
      }

      setProjectIds(Array.from(newProjectIds));
    } else {
      // Regular click behavior
      setProjectIds(
        projectIds.includes(projectId)
          ? projectIds.filter((id) => id !== projectId)
          : [...projectIds, projectId],
      );
    }
    setLastSelectedIndex(index);
  };

  return (
    <Command.Group heading="Select projects to delete">
      {isSubmitting && (
        <Command.Loading>
          <div className="flex items-center gap-1 text-sm text-content-secondary">
            <Spinner className="size-4" />
            Submitting...
          </div>
        </Command.Loading>
      )}
      {!isSubmitting &&
        projects?.map((project, index) => (
          <Command.Item
            key={project.id}
            className="flex justify-between"
            keywords={[project.name, project.slug]}
            onSelect={(event) =>
              toggleProject(
                project.id,
                index,
                event as unknown as React.MouseEvent,
              )
            }
          >
            <div className="flex items-center gap-1">
              <Checkbox
                className="mr-1"
                checked={projectIds.includes(project.id)}
                onChange={(event) =>
                  toggleProject(
                    project.id,
                    index,
                    event as unknown as React.MouseEvent,
                  )
                }
              />
              <p>
                {project.name}{" "}
                <span className="text-content-tertiary">({project.slug})</span>
              </p>
            </div>
            <TimestampDistance
              date={new Date(project.createTime)}
              prefix="Created"
            />
          </Command.Item>
        ))}
      {projectIds.length > 0 && (
        <Command.Item
          className={cn(
            buttonClasses({ size: "xs", variant: "neutral" }),
            "bottom-four absolute right-4 z-20 flex items-center gap-4 text-xs",
          )}
          data-button
          onSelect={handleDeleteProjects}
          forceMount
        >
          <TrashIcon className="size-4" />
          Delete {projectIds.length}{" "}
          {projectIds.length === 1 ? "project" : "projects"}
        </Command.Item>
      )}
    </Command.Group>
  );
}
