import { Command } from "cmdk";
import { TrashIcon, MagnifyingGlassIcon } from "@radix-ui/react-icons";
import {
  useCurrentProject,
  useDeleteProjects,
  useInfiniteProjects,
} from "api/projects";
import { useCurrentTeam } from "api/teams";
import React from "react";
import { Checkbox } from "@ui/Checkbox";
import { useHotkeys } from "react-hotkeys-hook";
import { Button } from "@ui/Button";
import { Spinner } from "@ui/Spinner";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { Tooltip } from "@ui/Tooltip";
import { useClickAway } from "react-use";
import { useRouter } from "next/router";
import { InfiniteScrollList } from "@common/elements/InfiniteScrollList";
import type { ProjectDetails } from "generatedApi";

export function CommandPalette() {
  const [open, setOpen] = React.useState(false);
  const [search, setSearch] = React.useState("");
  const [pages, setPages] = React.useState<string[]>([]);
  const page = pages[pages.length - 1];

  useHotkeys(["meta+k", "ctrl+k"], (event) => {
    event.preventDefault();
    setOpen((isOpen) => !isOpen);
  });

  useHotkeys(
    ["escape", "backspace"],
    (event) => {
      if (
        pages.length > 0 &&
        (event.key === "Escape" || (event.key === "Backspace" && !search))
      ) {
        event.preventDefault();
        setPages((currentPages) => currentPages.slice(0, -1));
      } else if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
      }
    },
    { enabled: open },
  );

  const ref = React.useRef<HTMLDivElement>(null);

  useClickAway(ref, () => {
    setOpen(false);
  });

  const isTeamAdmin = true;
  const { commandPalette, commandPaletteDeleteProjects } = useLaunchDarkly();

  if (!commandPalette) {
    return null;
  }

  if (open && page === "delete-projects") {
    return (
      <>
        <div
          className="fixed inset-0 z-40 bg-black/25"
          onClick={() => {
            setOpen(false);
            setPages([]);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              setOpen(false);
              setPages([]);
            }
          }}
          role="button"
          tabIndex={0}
          aria-label="Close command palette"
        />
        <div
          ref={ref}
          className="fixed top-1/3 left-1/2 z-50 w-full max-w-[640px] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-lg bg-background-secondary/95 p-2 shadow-md backdrop-blur-xs dark:border"
        >
          <DeleteProjectsPage
            onClose={() => {
              setOpen(false);
              setPages([]);
            }}
          />
        </div>
      </>
    );
  }

  return (
    <Command.Dialog
      open={open}
      ref={ref}
      label="Convex Command Palette"
      title="Convex Command Palette"
    >
      <Command.Input
        placeholder="What do you want to do?"
        value={search}
        onValueChange={setSearch}
      />
      <Command.List>
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
        <Command.Empty>No results found.</Command.Empty>
      </Command.List>
    </Command.Dialog>
  );
}

const DELETE_PROJECT_ITEM_SIZE = 44;

function DeleteProjectsPage({ onClose }: { onClose: () => void }) {
  const router = useRouter();
  const [projectIds, setProjectIds] = React.useState<number[]>([]);
  const [lastSelectedIndex, setLastSelectedIndex] = React.useState<
    number | null
  >(null);
  const [projectQuery, setProjectQuery] = React.useState("");

  const currentTeam = useCurrentTeam();
  const currentProject = useCurrentProject();

  // Use server-side search
  const { projects, hasMore, loadMore, isLoading, debouncedQuery, pageSize } =
    useInfiniteProjects(currentTeam?.id ?? 0, projectQuery);

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

  const toggleProject = React.useCallback(
    (projectId: number, index: number, event: React.MouseEvent) => {
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
    },
    [projectIds, lastSelectedIndex, projects],
  );

  const itemData = React.useMemo(
    () => ({
      projects: projects ?? [],
      projectIds,
      toggleProject,
    }),
    [projects, projectIds, toggleProject],
  );

  const itemKey = React.useMemo(
    () => (idx: number, data: typeof itemData) =>
      data.projects[idx]?.id?.toString() || `loading-${idx}`,
    [],
  );

  const scrollRef = React.useRef<HTMLDivElement>(null);

  return (
    <div className="flex flex-col">
      <div className="mb-2 flex items-center gap-2 rounded-sm rounded-b-none border-0 border-b bg-transparent px-3 py-2">
        {isLoading && debouncedQuery === projectQuery ? (
          <div className="animate-fadeInFromLoading">
            <Spinner className="size-3" />
          </div>
        ) : (
          <MagnifyingGlassIcon className="animate-fadeInFromLoading text-content-secondary" />
        )}

        <input
          autoFocus
          onChange={(e) => {
            setProjectQuery(e.target.value);
          }}
          value={projectQuery}
          className="w-full bg-transparent text-sm placeholder:text-content-tertiary focus:outline-hidden"
          placeholder="Search projects..."
        />
      </div>

      <div className="px-2 py-1 text-xs text-content-tertiary select-none">
        Select projects to delete
      </div>

      {isSubmitting ? (
        <div className="flex h-12 items-center justify-center gap-1 text-sm whitespace-pre-wrap text-content-tertiary">
          <Spinner className="size-4" />
          Submitting...
        </div>
      ) : !isSubmitting &&
        projects &&
        projects.length === 0 &&
        !isLoading &&
        debouncedQuery === projectQuery ? (
        <div className="flex h-12 items-center justify-center text-sm whitespace-pre-wrap text-content-tertiary">
          {debouncedQuery.trim()
            ? "No projects match your search."
            : "No projects found."}
        </div>
      ) : (
        <div
          className="overflow-auto overscroll-contain transition-[height] duration-100 ease-[ease] focus:outline-hidden"
          style={{
            height: 330,
          }}
        >
          <InfiniteScrollList
            outerRef={scrollRef}
            items={projects ?? []}
            totalNumItems={
              hasMore ? (projects?.length ?? 0) + 1 : (projects?.length ?? 0)
            }
            itemSize={DELETE_PROJECT_ITEM_SIZE}
            itemData={itemData}
            RowOrLoading={DeleteProjectListItem}
            overscanCount={25}
            loadMoreThreshold={1}
            loadMore={loadMore}
            pageSize={pageSize}
            itemKey={itemKey}
          />
        </div>
      )}

      {projectIds.length > 0 && !isSubmitting && (
        <div className="mt-2 flex justify-end">
          <Button
            size="xs"
            variant="neutral"
            onClick={handleDeleteProjects}
            icon={<TrashIcon className="size-4" />}
          >
            Delete {projectIds.length}{" "}
            {projectIds.length === 1 ? "project" : "projects"}
          </Button>
        </div>
      )}
    </div>
  );
}

function DeleteProjectListItem({
  index,
  style,
  data,
}: {
  index: number;
  style: React.CSSProperties;
  data: {
    projects: ProjectDetails[];
    projectIds: number[];
    toggleProject: (
      projectId: number,
      index: number,
      event: React.MouseEvent,
    ) => void;
  };
}) {
  const { projects, projectIds, toggleProject } = data;
  const project = projects[index];

  // Handle loading state or missing project
  if (!project) {
    return <div style={style} />;
  }

  return (
    <div style={style} className="px-0.5">
      <div
        className="flex cursor-pointer items-center justify-between gap-1 rounded-sm p-2 text-sm select-none hover:bg-background-tertiary active:bg-background-tertiary"
        onClick={(event) => toggleProject(project.id, index, event)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            toggleProject(project.id, index, e as unknown as React.MouseEvent);
          }
        }}
        role="button"
        tabIndex={0}
      >
        <div className="flex items-center gap-2">
          <Checkbox
            checked={projectIds.includes(project.id)}
            onChange={(event) =>
              toggleProject(
                project.id,
                index,
                event as unknown as React.MouseEvent,
              )
            }
          />
          <span>
            {project.name}{" "}
            <span className="text-content-tertiary">({project.slug})</span>
          </span>
        </div>
        <TimestampDistance
          date={new Date(project.createTime)}
          prefix="Created"
        />
      </div>
    </div>
  );
}
