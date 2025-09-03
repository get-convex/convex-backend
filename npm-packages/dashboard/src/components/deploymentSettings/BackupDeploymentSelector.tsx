import { Listbox, Transition } from "@headlessui/react";
import { CaretSortIcon, ChevronLeftIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Loading } from "@ui/Loading";
import { Tooltip } from "@ui/Tooltip";
import { useDeployments } from "api/deployments";
import { useProjects } from "api/projects";
import { useProfile } from "api/profile";
import { cn } from "@ui/cn";
import { Fragment, useCallback, useMemo, useState } from "react";
import { DeploymentResponse, Team } from "generatedApi";
import { createPortal } from "react-dom";
import { usePopper } from "react-popper";
import { FullDeploymentName } from "./BackupListItem";

export function BackupDeploymentSelector({
  selectedDeployment,
  onChange,
  team,
  targetDeployment,
}: {
  selectedDeployment: DeploymentResponse;
  onChange: (newDeployment: DeploymentResponse) => void;
  team: Team;
  targetDeployment: DeploymentResponse;
}) {
  const projects = useProjects(team.id);

  const [selectedProjectId, setSelectedProjectId] = useState(
    selectedDeployment.projectId,
  );
  const selectedProject = projects?.find((p) => p.id === selectedProjectId);
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

  const rowCount =
    (currentPage === "projects"
      ? projects?.length
      : selectedProjectDeployments?.length) ?? 5;
  const heightRem = 3.5 + 2.25 * Math.min(rowCount, 9.5);

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
              : selectedDeployment.id
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
              <Listbox.Button
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
                {selectedDeployment.id === targetDeployment.id ? (
                  "Current Deployment"
                ) : (
                  <FullDeploymentName
                    deployment={selectedDeployment}
                    team={team}
                  />
                )}
                <span className="pointer-events-none flex items-center">
                  <CaretSortIcon
                    className={cn("text-content-primary", "ml-auto h-5 w-5")}
                    aria-hidden="true"
                  />
                </span>
              </Listbox.Button>
              {open &&
                createPortal(
                  <Transition
                    as={Fragment}
                    leave="transition ease-in duration-100"
                    leaveFrom="opacity-100"
                    leaveTo="opacity-0"
                  >
                    <Listbox.Options
                      as="div"
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

                            <div className="grow overflow-x-hidden overflow-y-auto">
                              {projects === undefined ? (
                                <Loading />
                              ) : (
                                <ul className="p-0.5">
                                  {projects.map((project) => (
                                    <Listbox.Option
                                      key={project.id}
                                      value={project.id}
                                      className={({ active, selected }) =>
                                        cn(
                                          "flex w-full cursor-pointer items-center rounded-sm p-2 text-left text-sm text-content-primary hover:bg-background-tertiary",
                                          active && "bg-background-tertiary",
                                          selected &&
                                            "bg-background-tertiary/60",
                                        )
                                      }
                                      disabled={currentPage !== "projects"}
                                    >
                                      <span className="w-full truncate">
                                        {project.name}
                                      </span>
                                    </Listbox.Option>
                                  ))}
                                </ul>
                              )}
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
                                    (deployment) => (
                                      <Tooltip
                                        className="w-full"
                                        key={deployment.id}
                                        tip={<code>{deployment.name}</code>}
                                        side="right"
                                      >
                                        <Listbox.Option
                                          as="div"
                                          value={deployment.id}
                                          className={({ active, selected }) =>
                                            cn(
                                              "flex w-full cursor-pointer items-center rounded-sm p-2 text-left text-sm text-content-primary hover:bg-background-tertiary",
                                              active &&
                                                "bg-background-tertiary",
                                              selected &&
                                                "bg-background-tertiary/60",
                                            )
                                          }
                                          disabled={
                                            currentPage !== "deployments"
                                          }
                                        >
                                          <span className="w-full truncate">
                                            <FullDeploymentName
                                              deployment={deployment}
                                              team={team}
                                              showProjectName={false}
                                            />
                                          </span>
                                        </Listbox.Option>
                                      </Tooltip>
                                    ),
                                  )}
                                </div>
                              )}
                            </div>
                          </div>
                        </div>
                      </div>
                    </Listbox.Options>
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
  deployment: DeploymentResponse,
  isMine: boolean,
): number {
  const { deploymentType } = deployment;
  switch (deploymentType) {
    case "prod":
      return 0;
    case "preview":
      return 1;
    case "dev":
      return isMine ? 2 : 3;
    default: {
      deploymentType satisfies never;
      throw new Error("Unexpected deployment type");
    }
  }
}
