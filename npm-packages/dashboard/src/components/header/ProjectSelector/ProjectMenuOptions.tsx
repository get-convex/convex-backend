import { MagnifyingGlassIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { useCurrentProject } from "api/projects";
import { useState, useMemo, useRef } from "react";
import { TeamResponse, ProjectDetails } from "generatedApi";
import classNames from "classnames";
import { SelectorItem } from "elements/SelectorItem";
import { useDeploymentUris } from "hooks/useDeploymentUris";
import { useLastViewedDeploymentForProject } from "hooks/useLastViewed";
import { InfiniteScrollList } from "dashboard-common/src/elements/InfiniteScrollList";

const PROJECT_SELECTOR_ITEM_SIZE = 44;

export function ProjectMenuOptions({
  projectsForCurrentTeam,
  team,
  onCreateProjectClick,
  close,
}: {
  projectsForCurrentTeam?: ProjectDetails[];
  team: TeamResponse;
  onCreateProjectClick: (team: TeamResponse) => void;
  close(): void;
}) {
  const currentProject = useCurrentProject();
  const [projectQuery, setProjectQuery] = useState("");

  const items = useMemo(() => {
    if (!projectsForCurrentTeam) return [];

    const matchingProjects = projectsForCurrentTeam
      .filter((p) => p.name?.toLowerCase().includes(projectQuery.toLowerCase()))
      .reverse();

    const result = [];

    if (
      currentProject?.name.toLowerCase().includes(projectQuery.toLowerCase())
    ) {
      result.push({ ...currentProject, _isCurrent: true });
    }

    result.push(
      ...matchingProjects.filter((p) => p.slug !== currentProject?.slug),
    );

    return result;
  }, [projectsForCurrentTeam, projectQuery, currentProject]);

  const scrollRef = useRef<HTMLDivElement>(null);

  return (
    <>
      <div className="sticky top-0 z-10 flex w-full items-center gap-2 border-b bg-background-secondary px-3">
        <MagnifyingGlassIcon className="text-content-secondary" />
        <input
          autoFocus
          onChange={(e) => {
            setProjectQuery(e.target.value);
          }}
          value={projectQuery}
          className={classNames(
            "placeholder:text-content-tertiary truncate relative w-full py-1.5 text-left text-xs text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
            "focus:outline-hidden bg-background-secondary font-normal",
          )}
          placeholder="Search projects..."
        />
      </div>
      <label
        className="px-2 pt-1 text-xs font-semibold text-content-secondary"
        htmlFor="project-menu-options"
      >
        Projects
      </label>
      <div
        id="project-menu-options"
        className="w-full"
        style={{
          height: Math.min(items.length * PROJECT_SELECTOR_ITEM_SIZE, 22 * 16),
        }}
        role="menu"
      >
        <InfiniteScrollList
          outerRef={scrollRef}
          items={items}
          totalNumItems={items.length}
          itemSize={PROJECT_SELECTOR_ITEM_SIZE}
          itemData={{
            items,
            team,
            close,
          }}
          RowOrLoading={ProjectSelectorListItem}
          overscanCount={25}
          itemKey={(idx, data) =>
            data.items[idx]?.id?.toString() || `loading-${idx}`
          }
        />
      </div>
      <Button
        inline
        onClick={() => {
          onCreateProjectClick(team);
          close();
        }}
        icon={<PlusIcon aria-hidden="true" />}
        className="w-full"
        size="sm"
      >
        Create Project
      </Button>
    </>
  );
}

function ProjectSelectorListItem({
  index,
  style,
  data,
}: {
  index: number;
  style: React.CSSProperties;
  data: {
    items: (ProjectDetails & { _isCurrent?: boolean })[];
    team: TeamResponse;
    close: () => void;
  };
}) {
  const { items, team, close } = data;
  const project = items[index];
  // _isCurrent is only set for the current project
  const selected = Boolean(project._isCurrent);
  return (
    <div style={style}>
      <ProjectSelectorItem
        selected={selected}
        close={close}
        project={project}
        key={project.id}
        teamSlug={team.slug}
      />
    </div>
  );
}

function ProjectSelectorItem({
  project,
  teamSlug,
  close,
  selected = false,
  active = false,
  onFocusOrMouseEnter,
  optionRef,
}: {
  project: ProjectDetails;
  teamSlug?: string;
  close: () => void;
  selected?: boolean;
  active?: boolean;
  onFocusOrMouseEnter?: () => void;
  optionRef?: React.RefObject<HTMLDivElement>;
}) {
  const { generateHref, defaultHref } = useDeploymentUris(
    project.id,
    project.slug,
    teamSlug,
  );
  const [lastViewedDeployment] = useLastViewedDeploymentForProject(
    project.slug,
  );
  return (
    <div
      className="flex w-full gap-0.5 p-0.5"
      style={{
        height: PROJECT_SELECTOR_ITEM_SIZE,
      }}
      ref={active ? optionRef : undefined}
    >
      <SelectorItem
        className="grow"
        href={
          lastViewedDeployment
            ? generateHref(lastViewedDeployment)
            : defaultHref!
        }
        active={active}
        selected={selected}
        close={close}
        key={project.slug}
        onFocusOrMouseEnter={onFocusOrMouseEnter}
        eventName="switch project"
      >
        <span className="truncate">{project.name}</span>
      </SelectorItem>
    </div>
  );
}
