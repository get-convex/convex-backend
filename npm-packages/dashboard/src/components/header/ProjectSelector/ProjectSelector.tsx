import { ProjectDetails, Team } from "generatedApi";

import classNames from "classnames";
import React, { useRef, useState } from "react";
import { useCurrentProject, useProjects } from "api/projects";
import { Button } from "@ui/Button";
import { Popover } from "@ui/Popover";
import { CaretSortIcon, GearIcon, ResetIcon } from "@radix-ui/react-icons";
import { Avatar } from "elements/Avatar";
import { useScrolling, useWindowSize } from "react-use";
import { usePopper } from "react-popper";
import { cn } from "@ui/cn";
import { logEvent } from "convex-analytics";
import { SafeZone } from "elements/SafeZone";
import { DeploymentDisplay } from "elements/DeploymentDisplay";
import { Breadcrumbs } from "../Breadcrumbs/Breadcrumbs";
import { DeploymentMenuOptions } from "./DeploymentMenuOptions";
import { ProjectMenuOptions } from "./ProjectMenuOptions";
import { TeamMenuOptions } from "./TeamMenuOptions";

export function ProjectSelector({
  teams,
  className,
  selectedTeamSlug,
  selectedProject,
  onCreateProjectClick,
  onCreateTeamClick,
}: {
  teams?: Team[];
  selectedTeamSlug?: string;
  selectedProject?: ProjectDetails;
  className?: string;
  onCreateTeamClick: () => void;
  onCreateProjectClick: (team: Team) => void;
}) {
  const team = teams?.find((t) => t.slug === selectedTeamSlug) ?? null;

  const projectsForHoveredTeam = useProjects(team?.id);

  const currentProject = useCurrentProject();

  const [lastHoveredProject, setLastHoveredProject] =
    useState<ProjectDetails | null>(currentProject || null);

  const { width } = useWindowSize();

  const selected =
    team === undefined ? null : (
      <Breadcrumbs>
        {team && selectedProject ? (
          <Avatar name={team.name} hashKey={team.id.toString()} />
        ) : null}
        {selectedProject ? (
          <div
            className="truncate"
            style={{
              maxWidth: width > 1024 ? "14rem" : width > 640 ? "10rem" : "6rem",
            }}
          >
            {selectedProject.name}
          </div>
        ) : null}
        {selectedProject ? (
          <DeploymentDisplay project={selectedProject} />
        ) : (
          <div
            className="flex max-w-[14rem] items-center gap-2"
            style={{
              maxWidth: width > 1024 ? "14rem" : width > 640 ? "10rem" : "6rem",
            }}
          >
            <Avatar name={team?.name} hashKey={team?.id.toString() ?? ""} />
            <span className="grow truncate">{team?.name}</span>
          </div>
        )}
      </Breadcrumbs>
    );

  const button = (
    <Button
      aria-label="Switch to team selection"
      variant="unstyled"
      type="button"
      className={classNames(
        "rounded",
        "items-center h-12",
        "px-3 py-2 w-fit flex gap-2 select-none",
        ...(className !== undefined
          ? [className]
          : ["text-content-primary", "hover:bg-background-tertiary"]),
      )}
      onClick={() => {
        logEvent("click project selector");
      }}
    >
      {selected}
      <CaretSortIcon className="size-5" />
    </Button>
  );

  return (
    <Popover
      padding={false}
      focus
      className="-mt-0.5"
      portal
      placement="bottom-start"
      openButtonClassName="bg-background-tertiary rounded"
      onClose={() => {
        setLastHoveredProject(selectedProject || null);
      }}
      button={button}
    >
      {({ close }) => (
        <ProjectSelectorPanel
          close={close}
          teams={teams}
          onCreateTeamClick={onCreateTeamClick}
          onCreateProjectClick={onCreateProjectClick}
          team={team}
          projectsForHoveredTeam={projectsForHoveredTeam}
          lastHoveredProject={lastHoveredProject}
          setLastHoveredProject={setLastHoveredProject}
        />
      )}
    </Popover>
  );
}

function ProjectSelectorPanel({
  teams,
  onCreateTeamClick,
  onCreateProjectClick,
  close,
  team,
  projectsForHoveredTeam,
  lastHoveredProject,
  setLastHoveredProject,
}: {
  teams?: Team[];
  onCreateTeamClick: () => void;
  onCreateProjectClick: (team: Team) => void;
  close: () => void;
  team: Team | null;
  projectsForHoveredTeam: ProjectDetails[] | undefined;
  lastHoveredProject: ProjectDetails | null;
  setLastHoveredProject: (project: ProjectDetails | null) => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const optionsRef = useRef<HTMLDivElement>(null);
  const [popperElement, setPopperElement] = useState<HTMLDivElement | null>(
    null,
  );
  const { styles, attributes } = usePopper(optionsRef.current, popperElement, {
    placement: "right-start",
  });
  const isScrolling = useScrolling(scrollRef);

  const [switchingTeams, setSwitchingTeams] = useState(false);

  const [isInSafeZone, setIsInSafeZone] = useState(false);

  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions
    <div
      ref={menuRef}
      role="dialog"
      onKeyDown={(event) => {
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
          return;
        }

        const elementToFocus =
          event.key === "ArrowRight"
            ? popperElement?.querySelectorAll<HTMLAnchorElement>(
                ".SelectorItem:not([disabled])",
              )[0]
            : (popperElement?.parentElement!.children[0].querySelector(
                ".SelectorItem-active",
              ) as HTMLElement);
        elementToFocus?.focus();
      }}
    >
      {team && (
        <div className="flex max-h-[calc(100vh-3.625rem)] w-[12rem] flex-col py-2 sm:h-fit sm:w-[21.5rem]">
          <div className="my-0.5 flex w-full items-center justify-between gap-2 px-0.5">
            <h5 className="flex h-full items-center gap-1 truncate">
              {switchingTeams ? (
                <div className="px-1.5 py-2 text-sm">Select Team</div>
              ) : (
                <Button
                  variant="unstyled"
                  className="group flex items-center gap-1 px-1.5 py-2"
                  onClick={() => setSwitchingTeams(true)}
                  onMouseOver={() => setLastHoveredProject(null)}
                  tip="Select team"
                  tipSide="right"
                >
                  <Avatar name={team.name} hashKey={team.id.toString()} />
                  <span className="max-w-[12rem] truncate">{team.name}</span>
                  <CaretSortIcon
                    className={cn(
                      "text-content-primary",
                      "min-h-[1rem] min-w-[1rem] group-hover:bg-background-tertiary rounded-full",
                    )}
                  />
                </Button>
              )}
            </h5>
            {switchingTeams ? (
              <Button
                size="xs"
                onClick={() => setSwitchingTeams(false)}
                inline
                variant="neutral"
                icon={<ResetIcon />}
                tip="Select project"
                aria-label={`Switch to project selection for ${team.name}`}
                tipSide="right"
              />
            ) : (
              <Button
                size="xs"
                href={`/t/${team.slug}/settings`}
                onClickOfAnchorLink={close}
                inline
                variant="neutral"
                icon={<GearIcon />}
                tip="Team settings"
                aria-label={`Team settings for ${team.name}`}
                tipSide="right"
              />
            )}
          </div>
          <div className="flex flex-col items-start gap-0.5 overflow-y-auto overflow-x-hidden scrollbar sm:max-h-[22rem]">
            {switchingTeams ? (
              <TeamMenuOptions
                teams={teams}
                close={close}
                onCreateTeamClick={onCreateTeamClick}
                team={team}
              />
            ) : (
              <ProjectMenuOptions
                onCreateProjectClick={onCreateProjectClick}
                projectsForHoveredTeam={projectsForHoveredTeam}
                lastHoveredProject={lastHoveredProject}
                team={team}
                setLastHoveredProject={(p) =>
                  !isInSafeZone && setLastHoveredProject(p)
                }
                optionRef={optionsRef}
                scrollRef={scrollRef}
                close={close}
              />
            )}
          </div>
        </div>
      )}
      {menuRef.current && popperElement && (
        <SafeZone
          anchor={menuRef.current}
          submenu={popperElement}
          setIsInSafeZone={setIsInSafeZone}
        />
      )}
      {!isScrolling &&
        !switchingTeams &&
        team &&
        lastHoveredProject &&
        !lastHoveredProject.isDemo && (
          <div
            key={lastHoveredProject.id}
            ref={setPopperElement}
            style={styles.popper}
            className="max-h-[30rem] min-w-[8rem] max-w-[12rem] overflow-y-auto rounded border bg-background-secondary shadow-sm scrollbar sm:min-w-[12rem] sm:max-w-[20rem]"
            {...attributes.popper}
          >
            <div className="flex items-center justify-between gap-2 px-2 pt-2">
              <p className="truncate text-xs font-semibold text-content-secondary">
                Deployments
              </p>
              <Button
                size="xs"
                href={`/t/${team.slug}/${lastHoveredProject.slug}/settings`}
                onClickOfAnchorLink={close}
                inline
                variant="neutral"
                icon={<GearIcon />}
                tip={`Project settings for ${lastHoveredProject.slug}`}
              />
            </div>
            <DeploymentMenuOptions
              team={team}
              project={lastHoveredProject}
              close={close}
            />
          </div>
        )}
    </div>
  );
}
