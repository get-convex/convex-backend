import {
  ArchiveIcon,
  ArrowDownIcon,
  ArrowRightIcon,
  CrossCircledIcon,
  DotsVerticalIcon,
  ExclamationTriangleIcon,
  GlobeIcon,
  MinusCircledIcon,
  Pencil2Icon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Loading } from "@ui/Loading";
import { Spinner } from "@ui/Spinner";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { toast } from "@common/lib/utils";
import { Callout } from "@ui/Callout";
import { Modal } from "@ui/Modal";
import { Checkbox } from "@ui/Checkbox";
import { Menu, MenuItem } from "@ui/Menu";
import { useEffect, useId, useRef, useState } from "react";
import { DeploymentResponse, ProjectDetails, Team } from "generatedApi";
import { useDeploymentById } from "api/deployments";
import { useTeamMembers } from "api/teams";
import { useProjects } from "api/projects";
import { useProfile } from "api/profile";
import {
  CommandLineIcon,
  ServerIcon,
  SignalIcon,
} from "@heroicons/react/24/outline";
import {
  useRequestCloudBackup,
  useRestoreFromCloudBackup,
  useDeleteCloudBackup,
  BackupResponse,
  useListCloudBackups,
  useCancelCloudBackup,
} from "api/backups";
import { Doc, Id } from "system-udfs/convex/_generated/dataModel";
import { BackupIdentifier } from "elements/BackupIdentifier";
import { cn } from "@ui/cn";
import { getDeploymentLabel } from "elements/DeploymentDisplay";

export function BackupListItem({
  backup,
  restoring,
  someBackupInProgress,
  someRestoreInProgress,
  latestBackupInTargetDeployment,
  targetDeployment,
  team,
  canPerformActions,
  getZipExportUrl,
  maxCloudBackups,
  progressMessage,
}: {
  backup: BackupResponse;
  restoring: boolean;
  someBackupInProgress: boolean;
  someRestoreInProgress: boolean;
  latestBackupInTargetDeployment: BackupResponse | null;
  targetDeployment: DeploymentResponse;
  team: Team;
  canPerformActions: boolean;
  getZipExportUrl: (snapshotId: Id<"_exports">) => string;
  maxCloudBackups: number;
  progressMessage: string | null;
}) {
  const [modal, setModal] = useState<
    null | "suggestBackup" | "restoreConfirmation" | "delete" | "cancel"
  >(null);

  const previousState = useRef(backup.state);
  useEffect(() => {
    if (
      previousState.current !== "complete" &&
      backup.state === "complete" &&
      backup.sourceDeploymentId === targetDeployment.id
    ) {
      toast("success", "Backup completed successfully.");
    }
    previousState.current = backup.state;
  }, [backup.state, backup.sourceDeploymentId, targetDeployment.id]);

  let backupStateDescription;
  if (backup.state === "requested" || backup.state === "inProgress") {
    backupStateDescription = "This backup hasn't completed yet.";
  } else if (backup.state === "complete") {
    backupStateDescription = "This backup is complete.";
  } else if (backup.state === "canceled") {
    backupStateDescription = "This backup was canceled.";
  } else if (backup.state === "failed") {
    backupStateDescription = "This backup failed.";
  } else {
    const _: never = backup.state;
  }

  return (
    <>
      <div className="flex w-full flex-col">
        <div className="my-2 flex flex-wrap items-center gap-4">
          <div className="flex flex-1 flex-col items-start gap-1">
            <span className="min-w-fit text-sm font-medium">
              Backup from {new Date(backup.requestedTime).toLocaleString()}{" "}
              <span className="text-xs font-normal text-content-secondary">
                (<TimestampDistance date={new Date(backup.requestedTime)} />)
              </span>
            </span>
            <BackupIdentifier backup={backup} />
            {(backup.state === "requested" ||
              backup.state === "inProgress") && (
              <div className="flex items-center gap-2 text-content-secondary">
                <Spinner />
                {progressMessage || "In progress"}
              </div>
            )}
          </div>
          {backup.expirationTime !== null && (
            <TimestampDistance
              prefix="Expires "
              date={new Date(backup.expirationTime)}
              className="text-left text-content-errorSecondary"
            />
          )}
          {backup.state === "failed" && (
            <Tooltip
              tip="This backup couldn’t be completed. Contact support@convex.dev for help."
              side="right"
            >
              <div className="flex items-center gap-1 text-content-errorSecondary">
                <CrossCircledIcon />
                <span className="border-b border-dotted border-b-content-errorSecondary/50">
                  Failed
                </span>
              </div>
            </Tooltip>
          )}
          {backup.state === "canceled" && (
            <div className="flex items-center gap-1 text-content-secondary">
              <MinusCircledIcon />
              Canceled
            </div>
          )}
          <div>
            {restoring ? (
              <div className="flex items-center gap-2 text-content-secondary">
                <Spinner />
                Restoring…
              </div>
            ) : (
              <Menu
                buttonProps={{
                  size: "xs",
                  variant: "neutral",
                  icon: <DotsVerticalIcon />,
                  title: "Backup options",
                }}
                placement="bottom-end"
              >
                {backup.state === "requested" ||
                backup.state === "inProgress" ? (
                  <MenuItem
                    variant="danger"
                    action={() => setModal("cancel")}
                    disabled={backup.state !== "inProgress"}
                    tipSide="left"
                    tip={
                      backup.state !== "inProgress"
                        ? "This backup hasn't started yet."
                        : !canPerformActions
                          ? "You do not have permission to cancel backups in production."
                          : undefined
                    }
                  >
                    Cancel
                  </MenuItem>
                ) : null}
                <MenuItem
                  disabled={
                    backup.state !== "complete" ||
                    backup.sourceDeploymentId !== targetDeployment.id
                  }
                  href={
                    backup.state === "complete"
                      ? getZipExportUrl(backup.snapshotId)
                      : "" // only when disabled
                  }
                  tipSide="left"
                  tip={
                    backup.state !== "complete"
                      ? backupStateDescription
                      : backup.sourceDeploymentId !== targetDeployment.id
                        ? "You may download this backup from the Backup & Restore page for the deployment in which this backup was created."
                        : null
                  }
                >
                  Download
                </MenuItem>
                <MenuItem
                  action={() => {
                    setModal(
                      latestBackupInTargetDeployment === null ||
                        !isInLastFiveMinutes(latestBackupInTargetDeployment)
                        ? "suggestBackup"
                        : "restoreConfirmation",
                    );
                  }}
                  disabled={
                    backup.state !== "complete" ||
                    someRestoreInProgress ||
                    someBackupInProgress ||
                    !canPerformActions
                  }
                  tipSide="left"
                  tip={
                    backup.state !== "complete"
                      ? backupStateDescription
                      : someRestoreInProgress
                        ? "Another backup is being restored at the moment."
                        : someBackupInProgress
                          ? "Please wait for the ongoing backup to be completed before restoring from a backup."
                          : !canPerformActions
                            ? "You do not have permission to restore backups in production."
                            : undefined
                  }
                >
                  Restore
                </MenuItem>
                <MenuItem
                  variant="danger"
                  action={() => setModal("delete")}
                  disabled={
                    backup.state === "inProgress" ||
                    backup.state === "requested" ||
                    !canPerformActions
                  }
                  tipSide="left"
                  tip={
                    backup.state === "inProgress" ||
                    backup.state === "requested"
                      ? backupStateDescription
                      : !canPerformActions
                        ? "You do not have permission to delete backups in production."
                        : undefined
                  }
                >
                  Delete
                </MenuItem>
              </Menu>
            )}
          </div>
        </div>
      </div>

      {(modal === "restoreConfirmation" || modal === "suggestBackup") && (
        <Modal
          onClose={() => setModal(null)}
          title={
            modal === "suggestBackup"
              ? "Backup before restoring?"
              : "Restore from a backup"
          }
          size="md"
        >
          {modal === "suggestBackup" ? (
            <SuggestBackup
              team={team}
              targetDeployment={targetDeployment}
              onClose={() => setModal(null)}
              onContinue={() => setModal("restoreConfirmation")}
              latestBackupInTargetDeployment={latestBackupInTargetDeployment}
              maxCloudBackups={maxCloudBackups}
              canPerformActions={canPerformActions}
            />
          ) : (
            <RestoreConfirmation
              backup={backup}
              targetDeployment={targetDeployment}
              team={team}
              latestBackupInTargetDeployment={latestBackupInTargetDeployment}
              onClose={() => setModal(null)}
            />
          )}
        </Modal>
      )}

      {(modal === "delete" || modal === "cancel") && (
        <DeleteOrCancelBackupModal
          action={modal}
          backup={backup}
          team={team}
          onClose={() => setModal(null)}
        />
      )}
    </>
  );
}

function SuggestBackup({
  team,
  targetDeployment,
  onClose,
  onContinue,
  latestBackupInTargetDeployment,
  maxCloudBackups,
  canPerformActions,
}: {
  team: Team;
  targetDeployment: DeploymentResponse;
  onClose: () => void;
  onContinue: () => void;
  latestBackupInTargetDeployment: BackupResponse | null;
  maxCloudBackups: number;
  canPerformActions: boolean;
}) {
  return (
    <>
      <p>
        Restoring the backup will erase the data currently in the deployment.
      </p>

      <DeploymentSummary
        deployment={targetDeployment}
        latestBackup={latestBackupInTargetDeployment}
        team={team}
      />

      <div className="flex justify-end gap-2">
        <BackupNowButton
          deployment={targetDeployment}
          team={team}
          maxCloudBackups={maxCloudBackups}
          canPerformActions={canPerformActions}
          onBackupRequested={onClose}
        />
        <Button variant="primary" onClick={onContinue}>
          Continue
        </Button>
      </div>
    </>
  );
}

function RestoreConfirmation({
  backup,
  targetDeployment,
  team,
  latestBackupInTargetDeployment,
  onClose,
}: {
  backup: BackupResponse;
  targetDeployment: DeploymentResponse;
  team: Team;
  latestBackupInTargetDeployment: BackupResponse | null;
  onClose: () => void;
}) {
  const requestRestore = useRestoreFromCloudBackup(targetDeployment.id);

  const [isSubmitting, setIsSubmitting] = useState(false);

  const needsCheckboxConfirmation = targetDeployment.deploymentType === "prod";
  const [checkboxConfirmation, setCheckboxConfirmation] = useState(false);
  const checkboxConfirmationId = useId();

  return (
    <>
      <TransferSummary
        backup={backup}
        targetDeployment={targetDeployment}
        latestBackupInTargetDeployment={latestBackupInTargetDeployment}
        team={team}
      />

      <p className="my-2">
        The data (tables and files) in <code>{targetDeployment.name}</code> will
        be replaced by the contents of the backup.
      </p>

      <p className="text-content-secondary">
        The rest of your deployment configuration (code, environment variables,
        scheduled functions, etc.) will not be changed.
      </p>

      {needsCheckboxConfirmation && (
        <Callout className="mt-4 w-fit">
          <label className="block" htmlFor={checkboxConfirmationId}>
            <Checkbox
              id={checkboxConfirmationId}
              className="mr-2"
              checked={checkboxConfirmation}
              onChange={() => setCheckboxConfirmation(!checkboxConfirmation)}
            />
            I understand that this will <strong>erase</strong> my current{" "}
            <strong>production data</strong>.
          </label>
        </Callout>
      )}

      <div className="mt-4 flex justify-end">
        <Button
          variant="neutral"
          onClick={async () => {
            setIsSubmitting(true);
            try {
              await requestRestore({
                id: backup.id,
              });
            } finally {
              setIsSubmitting(false);
            }

            onClose();
          }}
          disabled={needsCheckboxConfirmation && !checkboxConfirmation}
          loading={isSubmitting}
        >
          Restore
        </Button>
      </div>
    </>
  );
}

function DeleteOrCancelBackupModal({
  action,
  backup,
  team,
  onClose,
}: {
  action: "delete" | "cancel";
  backup: BackupResponse;
  team: Team;
  onClose: () => void;
}) {
  const doDelete = useDeleteCloudBackup(team.id, backup.id);
  const doCancel = useCancelCloudBackup(team.id, backup.id);

  const [isSubmitting, setIsSubmitting] = useState(false);

  return (
    <Modal
      onClose={onClose}
      title={action === "delete" ? "Delete backup" : "Cancel backup"}
    >
      <p className="text-content-secondary">This action cannot be undone.</p>

      <BackupSummary
        backup={backup}
        sourceDeploymentAppearance="inline"
        team={team}
      />

      <div className="flex justify-end gap-2">
        <Button
          variant="danger"
          onClick={async () => {
            setIsSubmitting(true);
            try {
              if (action === "delete") {
                await doDelete();
              } else if (action === "cancel") {
                await doCancel();
              } else {
                const _: never = action;
              }
            } finally {
              setIsSubmitting(false);
            }

            onClose();
          }}
          loading={isSubmitting}
        >
          {action === "delete" ? "Delete" : "Cancel"} Backup
        </Button>
      </div>
    </Modal>
  );
}

export type BackupSummaryProps = {
  backup: BackupResponse | null;
  sourceDeploymentAppearance: null | "inline" | "differentDeploymentWarning";
  team: Team;
};

function BackupSummary({
  backup,
  sourceDeploymentAppearance,
  team,
}: BackupSummaryProps) {
  const backupDeployment = useDeploymentById(
    team.id,
    backup?.sourceDeploymentId,
  );
  const sourceDeployment = backupDeployment ? (
    <Tooltip tip={<code>{backupDeployment.name}</code>}>
      <FullDeploymentName deployment={backupDeployment} team={team} />
    </Tooltip>
  ) : (
    <div className="h-16 min-w-52">
      <Loading />
    </div>
  );

  return (
    <div className="my-8 flex flex-col items-center gap-2">
      <p className="flex items-center gap-2 text-content-tertiary">
        <ArchiveIcon className="size-6" /> Backup
      </p>

      <div className="flex flex-col items-center gap-1 text-center">
        {backup !== null ? (
          <>
            <p>
              Backup from
              <br />
              {new Date(backup.requestedTime).toLocaleString()}
            </p>
            <p className="text-xs text-content-secondary">
              (<TimestampDistance date={new Date(backup.requestedTime)} />)
            </p>
          </>
        ) : (
          <em>Unknown backup</em>
        )}
      </div>

      {sourceDeploymentAppearance === "inline" && sourceDeployment}
      {sourceDeploymentAppearance === "differentDeploymentWarning" && (
        <div className="relative mt-4 rounded-md border border-util-warning px-6 py-3">
          <p className="absolute left-0 top-0 w-full -translate-y-1/2 text-center text-xs text-yellow-700 dark:text-util-warning">
            <span className="inline-flex items-center justify-center gap-1 bg-background-secondary px-2 py-1">
              <ExclamationTriangleIcon className="size-4" />
              From a different deployment
            </span>
          </p>

          <div className="mt-2 flex flex-col place-items-center items-center gap-1 text-xs">
            {sourceDeployment}
          </div>
        </div>
      )}
    </div>
  );
}

type DeploymentSummaryProps = {
  deployment: DeploymentResponse;
  // null = show no backup warning, undefined = show nothing
  latestBackup: BackupResponse | null | undefined;
  team: Team;
};

function DeploymentSummary({
  deployment,
  latestBackup,
  team,
}: DeploymentSummaryProps) {
  return (
    <div className="my-8 flex flex-col items-center gap-2">
      <p className="flex items-center gap-2 text-content-tertiary">
        <ServerIcon className="size-6" /> Deployment
      </p>
      <Tooltip tip={<code>{deployment.name}</code>}>
        <FullDeploymentName deployment={deployment} team={team} />
      </Tooltip>
      {latestBackup !== undefined && <LatestBackup backup={latestBackup} />}
    </div>
  );
}

export function TransferSummary({
  backup,
  targetDeployment,
  latestBackupInTargetDeployment,
  team,
}: {
  backup: BackupResponse | null;
  targetDeployment: DeploymentResponse;
  latestBackupInTargetDeployment: BackupResponse | null | undefined;
  team: Team;
}) {
  return (
    <div className="grid justify-center gap-2 md:flex md:gap-5">
      <BackupSummary
        backup={backup}
        sourceDeploymentAppearance={
          backup && backup.sourceDeploymentId !== targetDeployment.id
            ? "differentDeploymentWarning"
            : null
        }
        team={team}
      />

      <div className="md:my-8">
        <ArrowDownIcon className="size-6 text-content-tertiary md:hidden" />
        <ArrowRightIcon className="hidden size-6 text-content-tertiary md:block" />
      </div>

      <DeploymentSummary
        deployment={targetDeployment}
        team={team}
        latestBackup={latestBackupInTargetDeployment}
      />
    </div>
  );
}

export function FullDeploymentName({
  deployment,
  team,
  showProjectName = true,
}: {
  deployment: DeploymentResponse;
  team: Team;
  showProjectName?: boolean;
}) {
  const projects = useProjects(team.id);

  const project = projects?.find((p) => p.id === deployment.projectId);
  if (projects !== undefined && !project) {
    throw new Error("Unknown project");
  }

  const whoseName = useMemberName(project, deployment);
  return (
    <div className="flex flex-wrap items-center gap-2">
      {showProjectName && (
        <>
          {project === undefined ? (
            <span className="inline-block h-6 w-32">
              <Loading />
            </span>
          ) : (
            <span>{project.name}</span>
          )}
          <span className="text-content-secondary">/</span>
        </>
      )}
      <DeploymentLabel deployment={deployment} whoseName={whoseName ?? null} />
    </div>
  );
}

function useMemberName(
  project: ProjectDetails | undefined,
  deployment: DeploymentResponse | undefined,
) {
  const teamMembers = useTeamMembers(project?.teamId);
  const whose = teamMembers?.find((tm) => tm.id === deployment?.creator);
  const profile = useProfile();
  const whoseName =
    whose?.email === profile?.email
      ? profile?.name || profile?.email
      : whose?.name || whose?.email || "Teammate";
  return whoseName;
}

function LatestBackup({ backup }: { backup: BackupResponse | null }) {
  return (
    <p>
      {backup === null ? (
        <span className="text-xs text-content-errorSecondary">No backup</span>
      ) : (
        <TimestampDistance
          prefix="Last backup created"
          className="text-inherit"
          date={new Date(backup.requestedTime)}
        />
      )}
    </p>
  );
}

function isInLastFiveMinutes(backup: BackupResponse): boolean {
  const fiveMinutes = 5 * 60 * 1000;
  return backup.requestedTime >= Date.now() - fiveMinutes;
}

export function BackupNowButton({
  deployment,
  team,
  maxCloudBackups,
  canPerformActions,
  onBackupRequested,
}: {
  deployment: DeploymentResponse;
  team: Team;
  maxCloudBackups: number;
  canPerformActions: boolean;
  onBackupRequested?: () => void;
}) {
  const backups = useListCloudBackups(team?.id);
  const nonFailedBackupsForDeployment = backups?.filter(
    (backup) =>
      backup.sourceDeploymentName === deployment.name &&
      (backup.state === "requested" ||
        backup.state === "inProgress" ||
        backup.state === "complete"),
  );

  const requestBackup = useRequestCloudBackup(deployment.id, team.id);
  const [isOngoing, setIsOngoing] = useState(false);

  const doBackup = async () => {
    setIsOngoing(true);
    try {
      await requestBackup();
    } finally {
      setIsOngoing(false);
    }
  };

  return (
    <Button
      variant="neutral"
      className="w-fit"
      loading={isOngoing}
      icon={<ArchiveIcon />}
      onClick={async () => {
        await doBackup();
        if (onBackupRequested) {
          onBackupRequested();
        }
      }}
      disabled={
        nonFailedBackupsForDeployment === undefined ||
        nonFailedBackupsForDeployment.length >= maxCloudBackups ||
        !canPerformActions
      }
      tip={
        isOngoing
          ? "A backup is currently in progress."
          : nonFailedBackupsForDeployment &&
              nonFailedBackupsForDeployment.length >= maxCloudBackups
            ? `You can only have up to ${maxCloudBackups} backups on your current plan. Delete some of your existing backups in this deployment to create a new one.`
            : !canPerformActions
              ? "You do not have permission to create backups in production."
              : undefined
      }
    >
      Backup Now
    </Button>
  );
}

export function progressMessageForBackup(
  // Backup response from big-brain
  backup: BackupResponse,
  // Existing cloud backup within the backend
  existingCloudBackup: Doc<"_exports"> | null | undefined,
) {
  return existingCloudBackup?.state === "in_progress" &&
    backup.state === "inProgress" &&
    existingCloudBackup._id === backup.snapshotId
    ? existingCloudBackup.progress_message || null
    : null;
}

function DeploymentLabel({
  whoseName,
  deployment,
}: {
  deployment: DeploymentResponse;
  whoseName: string | null;
}) {
  return (
    <div className={cn("flex items-center gap-2 rounded-md")}>
      {deployment.deploymentType === "dev" ? (
        deployment.kind === "local" ? (
          <CommandLineIcon className="size-4" />
        ) : (
          <GlobeIcon className="size-4" />
        )
      ) : deployment.deploymentType === "prod" ? (
        <SignalIcon className="size-4" />
      ) : deployment.deploymentType === "preview" ? (
        <Pencil2Icon className="size-4" />
      ) : null}
      {getDeploymentLabel({
        deployment,
        whoseName,
      })}
    </div>
  );
}
