import { MagnifyingGlassIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "dashboard-common";
import { useCurrentProject } from "api/projects";
import { useState } from "react";
import { Team, ProjectDetails } from "generatedApi";
import classNames from "classnames";
import { SelectorItem } from "elements/SelectorItem";
import { useDeploymentUris } from "hooks/useDeploymentUris";

export function ProjectMenuOptions({
  projectsForHoveredTeam,
  lastHoveredProject,
  team,
  setLastHoveredProject,
  onCreateProjectClick,
  optionRef,
  scrollRef,
  close,
}: {
  projectsForHoveredTeam?: ProjectDetails[];
  lastHoveredProject: ProjectDetails | null;
  team: Team;
  setLastHoveredProject: (project: ProjectDetails | null) => void;
  onCreateProjectClick: (team: Team) => void;
  optionRef: React.RefObject<HTMLDivElement>;
  scrollRef: React.RefObject<HTMLDivElement>;
  close(): void;
}) {
  const currentProject = useCurrentProject();

  const [projectQuery, setProjectQuery] = useState("");

  return (
    <>
      <div className="sticky top-0 z-10 flex w-full items-center gap-2 border-b bg-background-secondary px-3">
        <MagnifyingGlassIcon className="text-content-secondary" />
        <input
          autoFocus
          onChange={(e) => {
            setProjectQuery(e.target.value);
            setLastHoveredProject(null);
          }}
          value={projectQuery}
          className={classNames(
            "placeholder:text-content-tertiary truncate relative w-full py-1.5 text-left text-xs text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
            "focus:outline-none bg-background-secondary font-normal",
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
      {projectsForHoveredTeam && (
        <div
          id="project-menu-options"
          className="flex w-full grow flex-col items-start overflow-y-auto p-0.5 scrollbar"
          role="menu"
          ref={scrollRef}
        >
          {currentProject &&
            currentProject.name
              .toLowerCase()
              .includes(projectQuery.toLowerCase()) && (
              <ProjectSelectorItem
                active={currentProject?.slug === lastHoveredProject?.slug}
                optionRef={optionRef}
                selected={currentProject.slug === currentProject?.slug}
                close={close}
                project={currentProject}
                key={currentProject.id}
                teamSlug={team.slug}
                onFocusOrMouseEnter={() =>
                  setLastHoveredProject(currentProject)
                }
              />
            )}
          {projectsForHoveredTeam
            .filter(
              (p) =>
                p.name?.toLowerCase().includes(projectQuery.toLowerCase()) &&
                p.slug !== currentProject?.slug,
            )
            .reverse()
            .map((project) => (
              <ProjectSelectorItem
                active={project.slug === lastHoveredProject?.slug}
                optionRef={optionRef}
                close={close}
                project={project}
                key={project.id}
                teamSlug={team.slug}
                onFocusOrMouseEnter={() => setLastHoveredProject(project)}
              />
            ))}
        </div>
      )}
      <Button
        inline
        onClick={() => {
          onCreateProjectClick(team);
          close();
        }}
        icon={<PlusIcon aria-hidden="true" />}
        className="w-full"
        onMouseOver={() => setLastHoveredProject(null)}
        size="sm"
      >
        Create Project
      </Button>
    </>
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
  const { defaultHref } = useDeploymentUris(project.id, project.slug, teamSlug);
  return (
    <div
      className={classNames("flex w-full gap-0.5 p-0.5")}
      ref={active ? optionRef : undefined}
    >
      <SelectorItem
        className="grow"
        href={defaultHref!}
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
