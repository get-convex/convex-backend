import {
  Listbox,
  ListboxButton,
  ListboxOptions,
  ListboxOption,
  Transition,
} from "@headlessui/react";
import {
  CaretSortIcon,
  ChevronLeftIcon,
  SewingPinFilledIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Loading } from "@ui/Loading";
import { Tooltip } from "@ui/Tooltip";
import { useDeployments } from "api/deployments";
import { useInfiniteProjects, useProjectById } from "api/projects";
import { useProfile } from "api/profile";
import { cn } from "@ui/cn";
import { useCallback, useMemo, useState } from "react";
import { TeamResponse } from "generatedApi";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { createPortal } from "react-dom";
import { usePopper } from "react-popper";
import { FullDeploymentName } from "./BackupListItem";

/** How many placeholder rows we show when loading more projects */
const LOADING_ROW_COUNT = 9;

export function BackupDeploymentSelector({
  selectedDeployment,
  onChange,
  team,
  targetDeployment,
}: {
  selectedDeployment: PlatformDeploymentResponse;
  onChange: (newDeployment: PlatformDeploymentResponse) => void;
  team: TeamResponse;
  targetDeployment: PlatformDeploymentResponse;
}) {
  const {
    projects,
    isLoading: isLoadingProjects,
    hasMore,
    loadMore,
  } = useInfiniteProjects(team.id);

  const currentProjectId =
    targetDeployment.kind === "cloud" ? targetDeployment.projectId : undefined;
  const { project: currentProject } = useProjectById(currentProjectId);

  // Filter out the current project from the paginated list because we display it at the top
  const paginatedProjects = useMemo(
    () => projects.filter((p) => p.id !== currentProjectId),
    [projects, currentProjectId],
  );

  const [selectedProjectId, setSelectedProjectId] = useState(
    selectedDeployment.projectId,
  );
  const selectedProject =
    currentProject?.id === selectedProjectId
      ? currentProject
      : paginatedProjects.find((p) => p.id === selectedProjectId);
  const { deployments } = useDeployments(selectedProjectId);

  const myProfile = useProfile();
  const selectedProjectDeployments = useMemo(() => {
    if (deployments === undefined) {
      return undefined;
    }

    const sorted = deployments.filter((d) => d.kind === "cloud");
    sorted.sort((a, b) => {
      const priorityA = deploymentListOrder(a, myProfile?.id === a.creator);
      const priorityB = deploymentListOrder(b, myProfile?.id === b.creator);
      return priorityA - priorityB;
    });
    return sorted;
  }, [deployments, myProfile]);

  const [currentPage, setCurrentPage] = useState<"projects" | "deployments">(
    "deployments",
  );
  const goBack = useCallback(() => {
    setCurrentPage("projects");

    // Reset the selection so that the selected item in the projects page
    // matches the selected deployment
    setSelectedProjectId(selectedDeployment.projectId);
  }, [selectedDeployment.projectId]);

  const showLoadingRows =
    hasMore || (isLoadingProjects && projects.length === 0);
  const projectRowCount =
    (currentProject ? 1 : 0) +
    paginatedProjects.length +
    (showLoadingRows ? LOADING_ROW_COUNT : 0);
  const rowCount =
    (currentPage === "projects"
      ? projectRowCount || 5
      : selectedProjectDeployments?.length) ?? 5;
  const heightRem = 3.5 + 2.25 * Math.min(rowCount, 9.5);

  const handleProjectsScroll = useCallback(
    (e: React.UIEvent<HTMLDivElement>) => {
      const el = e.currentTarget;
      if (
        el.scrollHeight - el.scrollTop - el.clientHeight <
        (LOADING_ROW_COUNT + 5) * 36
      ) {
        loadMore();
      }
    },
    [loadMore],
  );

  const [referenceElement, setReferenceElement] =
    useState<HTMLButtonElement | null>(null);
  const [popperElement, setPopperElement] = useState<HTMLDivElement | null>(
    null,
  );
  const { styles, attributes } = usePopper(referenceElement, popperElement, {
    placement: "bottom-start",
    modifiers: [{ name: "offset", options: { offset: [0, 8] } }],
  });

  return (
    <div className="flex w-full flex-wrap items-center justify-between gap-2 p-4">
      <h4 className="text-content-primary">Existing Backups</h4>
      {/* Listbox is used here to provide popovers with out-of-the-box keyboard navigation. */}
      <div className="relative">
        <Listbox
          // `multiple` is used here to prevent Headless from closing the popover
          // when a value is selected at the first level (project).
          multiple={currentPage === "projects"}
          value={
            currentPage === "projects"
              ? [selectedProjectId]
              : selectedDeployment.kind === "cloud"
                ? selectedDeployment.id
                : undefined
          }
          onChange={(eventValue) => {
            if (Array.isArray(eventValue)) {
              // Selected a project

              const projectId =
                // When the one-valued array changes, its new value will either
                // be [oldValue, selectedValue] or [] (when oldValue was selected)
                eventValue.at(eventValue.length - 1) ?? selectedProjectId;

              setSelectedProjectId(projectId);
              setCurrentPage("deployments");
            } else {
              // Selected a deployment

              const deploymentId = eventValue;
              onChange(
                selectedProjectDeployments!.find((d) => d.id === deploymentId)!,
              );
            }
          }}
        >
          {({ open }) => (
            <>
              <ListboxButton
                as={Button}
                ref={(el) =>
                  setReferenceElement(el as HTMLButtonElement | null)
                }
                variant="unstyled"
                className={cn(
                  "group relative flex items-center gap-1",
                  "truncate rounded-sm text-left text-content-primary disabled:cursor-not-allowed disabled:bg-background-tertiary disabled:text-content-secondary",
                  "border bg-background-secondary px-3 py-2 text-sm focus:border-border-selected focus:outline-hidden",
                  "hover:bg-background-tertiary",
                  open && "border-border-selected",
                  "cursor-pointer",
                  open && "bg-background-tertiary",
                )}
              >
                <span className="font-semibold">Restore from:</span>
                {selectedDeployment.kind === "cloud" &&
                targetDeployment.kind === "cloud" &&
                selectedDeployment.id === targetDeployment.id ? (
                  "Current Deployment"
                ) : (
                  <FullDeploymentName deployment={selectedDeployment} />
                )}
                <span className="pointer-events-none flex items-center">
                  <CaretSortIcon
                    className={cn("text-content-primary", "ml-auto h-5 w-5")}
                    aria-hidden="true"
                  />
                </span>
              </ListboxButton>
              {open &&
                createPortal(
                  <Transition
                    leave="transition ease-in duration-100"
                    leaveFrom="opacity-100"
                    leaveTo="opacity-0"
                  >
                    <ListboxOptions
                      ref={(el) =>
                        setPopperElement(el as HTMLDivElement | null)
                      }
                      {...attributes.popper}
                      className="absolute left-0 z-50 mt-2 w-64 overflow-hidden rounded-sm border bg-background-secondary shadow-sm transition-[max-height] focus:outline-hidden"
                      onKeyDown={(e) => {
                        switch (e.key) {
                          case "ArrowLeft":
                            goBack();
                            break;
                          case "ArrowRight":
                            e.key = "Enter"; // Will be handled by Headless UI as a current item selection
                            break;
                          default:
                        }
                      }}
                      style={{
                        ...styles.popper,
                        maxHeight: `${heightRem}rem`,
                      }}
                    >
                      <div
                        className={cn(
                          "flex h-full transition-transform duration-200 motion-reduce:transition-none",
                          currentPage === "deployments" && "-translate-x-full",
                        )}
                      >
                        {/* Projects */}
                        <div
                          // There are two pages, whose elements stay in the DOM even
                          // when they are not active (because there's a transition
                          // between the two).
                          // We disable all interactions (screen readers, find
                          // in page, events…) on pages that are not currently visible
                          // @ts-expect-error https://github.com/facebook/react/issues/17157
                          inert={
                            currentPage !== "projects" ? "inert" : undefined
                          }
                          className="w-64 shrink-0"
                          style={{ height: `${heightRem}rem` }}
                        >
                          <div className="flex h-full flex-col">
                            <header className="flex min-h-12 items-center border-b">
                              <span className="flex-1 truncate px-2 text-center font-semibold text-nowrap">
                                {team.name}
                              </span>
                            </header>

                            <div
                              className="grow overflow-x-hidden overflow-y-auto"
                              onScroll={handleProjectsScroll}
                            >
                              <ul className="p-0.5">
                                {currentProject && (
                                  <ListboxOption
                                    key={currentProject.id}
                                    value={currentProject.id}
                                    className={({ focus, selected }) =>
                                      cn(
                                        "flex w-full cursor-pointer items-center rounded-sm p-2 text-left text-sm text-content-primary hover:bg-background-tertiary",
                                        focus && "bg-background-tertiary",
                                        selected && "bg-background-tertiary/60",
                                      )
                                    }
                                    disabled={currentPage !== "projects"}
                                  >
                                    <span className="w-full truncate">
                                      {currentProject.name}
                                    </span>
                                    <Tooltip
                                      tip="This project"
                                      side="right"
                                      className="ml-auto"
                                    >
                                      <SewingPinFilledIcon className="min-h-[1rem] min-w-[1rem]" />
                                    </Tooltip>
                                  </ListboxOption>
                                )}
                                {paginatedProjects.map((project) => (
                                  <ListboxOption
                                    key={project.id}
                                    value={project.id}
                                    className={({ focus, selected }) =>
                                      cn(
                                        "flex w-full cursor-pointer items-center rounded-sm p-2 text-left text-sm text-content-primary hover:bg-background-tertiary",
                                        focus && "bg-background-tertiary",
                                        selected && "bg-background-tertiary/60",
                                      )
                                    }
                                    disabled={currentPage !== "projects"}
                                  >
                                    <span className="w-full truncate">
                                      {project.name}
                                    </span>
                                  </ListboxOption>
                                ))}
                                <div aria-label="Loading more projects">
                                  {showLoadingRows &&
                                    Array.from(
                                      { length: LOADING_ROW_COUNT },
                                      (_, i) => (
                                        <div
                                          key={`loading-${i}`}
                                          className="p-2 text-sm"
                                        >
                                          <Loading className="h-[1lh]" />
                                        </div>
                                      ),
                                    )}
                                </div>
                              </ul>
                            </div>
                          </div>
                        </div>

                        {/* Deployments */}
                        <div
                          // @ts-expect-error https://github.com/facebook/react/issues/17157
                          inert={
                            currentPage !== "deployments" ? "inert" : undefined
                          }
                          className="w-64 shrink-0"
                          style={{ height: `${heightRem}rem` }}
                        >
                          <div className="flex h-full flex-col">
                            <header className="flex min-h-12 w-full items-center border-b">
                              <Button
                                variant="unstyled"
                                className="flex h-full w-10 shrink-0 items-center justify-center rounded-sm text-content-secondary transition-colors hover:text-content-primary focus:outline-hidden"
                                onClick={() => goBack()}
                                tip="Back to projects"
                              >
                                <ChevronLeftIcon className="size-5" />
                              </Button>

                              {selectedProject ? (
                                <span className="mr-10 w-full truncate text-center font-semibold text-nowrap">
                                  {selectedProject.name}
                                </span>
                              ) : (
                                <div className="mr-10 flex w-full justify-center">
                                  <span className="h-6 w-36">
                                    <Loading />
                                  </span>
                                </div>
                              )}
                            </header>

                            <div className="grow overflow-x-hidden overflow-y-auto">
                              {selectedProjectDeployments === undefined ? (
                                <Loading />
                              ) : selectedProjectDeployments.length === 0 ? (
                                <div className="p-4 text-content-tertiary">
                                  This project has no cloud deployments.
                                </div>
                              ) : (
                                <div className="p-0.5">
                                  {selectedProjectDeployments.map(
                                    (deployment) => {
                                      const isRegionDifferent =
                                        targetDeployment.kind === "cloud" &&
                                        deployment.region !==
                                          targetDeployment.region;
                                      return (
                                        <Tooltip
                                          className="w-full"
                                          key={deployment.id}
                                          tip={
                                            isRegionDifferent ? (
                                              "Use the CLI to restore a backup from a deployment in a different region."
                                            ) : (
                                              <code>{deployment.name}</code>
                                            )
                                          }
                                          side="right"
                                        >
                                          <ListboxOption
                                            value={deployment.id}
                                            className={({ focus, selected }) =>
                                              cn(
                                                "flex w-full cursor-pointer items-center rounded-sm p-2 text-left text-sm text-content-primary hover:bg-background-tertiary",
                                                focus &&
                                                  "bg-background-tertiary",
                                                selected &&
                                                  "bg-background-tertiary/60",
                                              )
                                            }
                                            disabled={
                                              currentPage !== "deployments" ||
                                              isRegionDifferent
                                            }
                                          >
                                            <span className="w-full truncate">
                                              <FullDeploymentName
                                                deployment={deployment}
                                                showProjectName={false}
                                              />
                                            </span>
                                          </ListboxOption>
                                        </Tooltip>
                                      );
                                    },
                                  )}
                                </div>
                              )}
                            </div>
                          </div>
                        </div>
                      </div>
                    </ListboxOptions>
                  </Transition>,
                  document.body,
                )}
            </>
          )}
        </Listbox>
      </div>
    </div>
  );
}

function deploymentListOrder(
  deployment: PlatformDeploymentResponse,
  isMine: boolean,
): number {
  const { deploymentType } = deployment;
  switch (deploymentType) {
    case "prod":
      return 0;
    case "preview":
      return 1;
    case "custom":
      return 3;
    case "dev":
      return isMine ? 2 : 4;
    default: {
      deploymentType satisfies never;
      throw new Error("Unexpected deployment type");
    }
  }
}
