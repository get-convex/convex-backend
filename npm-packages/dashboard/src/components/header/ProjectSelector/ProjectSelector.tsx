import { ProjectDetails, Team } from "generatedApi";

import classNames from "classnames";
import React, { useState } from "react";
import { useProjects } from "api/projects";
import { Button } from "@ui/Button";
import { CaretSortIcon, GearIcon, ResetIcon } from "@radix-ui/react-icons";
import { Avatar } from "elements/Avatar";
import { cn } from "@ui/cn";
import { logEvent } from "convex-analytics";
import { useWindowSize } from "react-use";
import { Popover } from "@ui/Popover";
import { Breadcrumbs } from "../Breadcrumbs/Breadcrumbs";
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

  const projectsForCurrentTeam = useProjects(team?.id);

  const { width } = useWindowSize();

  const selected =
    team === undefined ? null : (
      <Breadcrumbs>
        {team && selectedProject ? (
          <Avatar name={team.name} hashKey={team.id.toString()} />
        ) : null}
        {selectedProject ? (
          <div
            className="truncate font-semibold"
            style={{
              maxWidth: width > 1024 ? "14rem" : width > 640 ? "10rem" : "6rem",
            }}
          >
            {selectedProject.name}
          </div>
        ) : null}
        {selectedProject ? null : (
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
        "items-center h-10",
        "px-3 py-2 w-fit flex gap-2 select-none",
        ...(className !== undefined
          ? [className]
          : ["text-content-primary", "hover:bg-background-tertiary"]),
        "rounded-full",
        "cursor-pointer",
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
      openButtonClassName="bg-background-tertiary rounded-full"
      button={button}
    >
      {({ close }) => (
        <ProjectSelectorPanel
          close={close}
          teams={teams}
          onCreateTeamClick={onCreateTeamClick}
          onCreateProjectClick={onCreateProjectClick}
          team={team}
          projectsForCurrentTeam={projectsForCurrentTeam}
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
  projectsForCurrentTeam,
}: {
  teams?: Team[];
  onCreateTeamClick: () => void;
  onCreateProjectClick: (team: Team) => void;
  close: () => void;
  team: Team | null;
  projectsForCurrentTeam: ProjectDetails[] | undefined;
}) {
  const [switchingTeams, setSwitchingTeams] = useState(false);

  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions
    <div role="dialog">
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
          <div className="flex flex-col items-start gap-0.5 overflow-x-hidden">
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
                projectsForCurrentTeam={projectsForCurrentTeam}
                team={team}
                close={close}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}
