import { MagnifyingGlassIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Spinner } from "@ui/Spinner";
import { useCurrentProject, useInfiniteProjects } from "api/projects";
import { useState, useMemo, useRef } from "react";
import { TeamResponse, ProjectDetails } from "generatedApi";
import classNames from "classnames";
import { SelectorItem } from "elements/SelectorItem";
import { useDeploymentUris } from "hooks/useDeploymentUris";
import { useLastViewedDeploymentForProject } from "hooks/useLastViewed";
import { InfiniteScrollList } from "@common/elements/InfiniteScrollList";
import { OpenInVercel } from "components/OpenInVercel";
import startCase from "lodash/startCase";

const PROJECT_SELECTOR_ITEM_SIZE = 44;

export function ProjectMenuOptions({
  team,
  onCreateProjectClick,
  close,
}: {
  team: TeamResponse;
  onCreateProjectClick: (team: TeamResponse) => void;
  close(): void;
}) {
  const currentProject = useCurrentProject();
  const [projectQuery, setProjectQuery] = useState("");

  const {
    projects: projectsForCurrentTeam,
    isLoading,
    hasMore,
    loadMore,
    debouncedQuery,
    pageSize,
  } = useInfiniteProjects(team.id, projectQuery);

  const items = useMemo(() => {
    if (!projectsForCurrentTeam || projectsForCurrentTeam.length === 0)
      return [];

    const result = [];

    // Check if current project is in the results
    const currentProjectInResults = projectsForCurrentTeam.find(
      (p) => p.slug === currentProject?.slug,
    );

    if (currentProjectInResults) {
      result.push({ ...currentProjectInResults, _isCurrent: true });
    }

    // Add all other projects
    result.push(
      ...projectsForCurrentTeam.filter((p) => p.slug !== currentProject?.slug),
    );

    return result;
  }, [projectsForCurrentTeam, currentProject?.slug]);

  const itemData = useMemo(
    () => ({
      items,
      team,
      close,
    }),
    [items, team, close],
  );

  const itemKey = useMemo(
    () => (idx: number, data: typeof itemData) =>
      data.items[idx]?.id?.toString() || `loading-${idx}`,
    [],
  );

  const scrollRef = useRef<HTMLDivElement>(null);

  return (
    <>
      <div className="sticky top-0 z-10 flex w-full items-center gap-2 border-b bg-background-secondary px-3">
        {isLoading && debouncedQuery === projectQuery ? (
          <div className="animate-fadeInFromLoading">
            <Spinner className="size-3" />
          </div>
        ) : (
          <MagnifyingGlassIcon className="animate-fadeInFromLoading text-content-secondary" />
        )}

        <div className="relative flex w-full items-center">
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
      </div>
      <label
        className="px-2 pt-1 text-xs font-semibold text-content-secondary"
        htmlFor="project-menu-options"
      >
        Projects
      </label>
      {items.length === 0 && !isLoading && debouncedQuery.trim() ? (
        <div className="flex w-full animate-fadeInFromLoading items-center justify-center py-4 text-xs text-content-secondary">
          No projects match your search.
        </div>
      ) : (
        <div
          id="project-menu-options"
          className="w-full"
          style={{
            height: Math.min(
              items.length * PROJECT_SELECTOR_ITEM_SIZE,
              22 * 16,
            ),
          }}
          role="menu"
        >
          <InfiniteScrollList
            outerRef={scrollRef}
            items={items}
            totalNumItems={hasMore ? items.length + 1 : items.length}
            itemSize={PROJECT_SELECTOR_ITEM_SIZE}
            itemData={itemData}
            RowOrLoading={ProjectSelectorListItem}
            overscanCount={25}
            loadMoreThreshold={1}
            loadMore={loadMore}
            pageSize={pageSize}
            itemKey={itemKey}
          />
        </div>
      )}

      <div className="flex w-full gap-2 p-2">
        <Button
          inline
          onClick={() => {
            onCreateProjectClick(team);
            close();
          }}
          icon={<PlusIcon aria-hidden="true" />}
          className="grow"
          size="sm"
          disabled={!!team.managedBy}
          tip={
            team.managedBy
              ? `This team is managed by ${startCase(team.managedBy)}. You can create new projects through the ${startCase(team.managedBy)} dashboard.`
              : ""
          }
        >
          Create Project
        </Button>
        <OpenInVercel team={team} />
      </div>
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

  // Handle loading state or missing project
  if (!project) {
    return <div style={style} />;
  }

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
