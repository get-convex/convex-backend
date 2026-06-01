import { ProjectDetails, TeamResponse } from "generatedApi";

import classNames from "classnames";
import React, { useState } from "react";
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
  selectedTeamSlug,
  selectedProject,
  onCreateProjectClick,
  onCreateTeamClick,
}: {
  teams?: TeamResponse[];
  selectedTeamSlug?: string;
  selectedProject?: ProjectDetails;
  onCreateTeamClick: () => void;
  onCreateProjectClick: (team: TeamResponse) => void;
}) {
  const team = teams?.find((t) => t.slug === selectedTeamSlug) ?? null;

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
            className="flex max-w-56 items-center gap-2"
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
        "flex items-center h-full",
        "w-fit select-none",
        "text-content-primary group",
        "cursor-pointer",
        "outline-none",
        "min-h-[calc(56px-1px)]", // navbar height - border
      )}
      onClick={() => {
        logEvent("click project selector");
      }}
    >
      <div
        className={classNames(
          "flex h-10 items-center px-3 py-2 rounded-full gap-2",
          "bg-(--project-selector-bg) group-hover:bg-background-tertiary",
          "group-focus-visible:ring-2 group-focus-visible:ring-inset group-focus-visible:ring-border-selected",
        )}
      >
        {selected}
        <CaretSortIcon className="size-5" />
      </div>
    </Button>
  );

  return (
    <Popover
      padding={false}
      focus
      className="-mt-2.5"
      portal
      placement="bottom-start"
      openButtonClassName="[--project-selector-bg:var(--background-tertiary)]"
      button={button}
    >
      {({ close }) => (
        <ProjectSelectorPanel
          close={close}
          teams={teams}
          onCreateTeamClick={onCreateTeamClick}
          onCreateProjectClick={onCreateProjectClick}
          team={team}
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
}: {
  teams?: TeamResponse[];
  onCreateTeamClick: () => void;
  onCreateProjectClick: (team: TeamResponse) => void;
  close: () => void;
  team: TeamResponse | null;
}) {
  const [switchingTeams, setSwitchingTeams] = useState(false);

  return (
    <div role="dialog">
      {team && (
        <div className="flex max-h-[calc(100vh-3.625rem)] w-48 flex-col py-2 sm:h-fit sm:w-86">
          <div className="my-0.5 flex w-full items-center justify-between gap-2 px-0.5">
            <h5 className="mb-1 flex h-fit items-center gap-1 truncate">
              {switchingTeams ? (
                <div className="px-1.5 py-2 text-sm">Select Team</div>
              ) : (
                <Button
                  variant="unstyled"
                  className="mx-1.5 flex cursor-pointer items-center gap-1 rounded-full border px-1.5 py-1 hover:bg-background-tertiary"
                  onClick={() => setSwitchingTeams(true)}
                  tip="Select team"
                  tipSide="right"
                >
                  <Avatar name={team.name} hashKey={team.id.toString()} />
                  <span className="max-w-48 truncate">{team.name}</span>
                  <CaretSortIcon
                    className={cn(
                      "text-content-primary",
                      "min-h-4 min-w-4 rounded-full",
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
