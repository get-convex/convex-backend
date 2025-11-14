import { DeploymentResponse, TeamResponse } from "generatedApi";
import { ArchiveIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { useGetZipExport } from "hooks/deploymentApi";
import { BackupResponse, useListCloudBackups } from "api/backups";
import { Loading } from "@ui/Loading";
import { EmptySection } from "@common/elements/EmptySection";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { BackupListItem, progressMessageForBackup } from "./BackupListItem";
import { BackupDeploymentSelector } from "./BackupDeploymentSelector";
import { useLatestRestore } from "./BackupRestoreStatus";

export function BackupList({
  targetDeployment,
  team,
  canPerformActions,
  maxCloudBackups,
}: {
  targetDeployment: DeploymentResponse; // = deployment the settings page is open for
  team: TeamResponse;
  canPerformActions: boolean;
  maxCloudBackups: number;
}) {
  const backups = useListCloudBackups(team?.id); // order: latest to oldest

  const [selectedDeployment, setSelectedDeployment] =
    useState(targetDeployment);

  const latestRestore = useLatestRestore();
  const requestor = latestRestore?.requestor;
  if (requestor && requestor.type !== "cloudRestore") {
    throw new Error("Unexpected requestor for restore");
  }

  const sourceCloudBackupId = requestor?.sourceCloudBackupId ?? null;

  const restoringBackupId =
    !!latestRestore &&
    !["completed", "failed"].includes(latestRestore.state.state)
      ? sourceCloudBackupId
      : null;

  return (
    <div className="flex h-full flex-col">
      <div className="border-b text-content-secondary">
        <BackupDeploymentSelector
          selectedDeployment={selectedDeployment}
          onChange={setSelectedDeployment}
          team={team}
          targetDeployment={targetDeployment}
        />
      </div>
      <div className="scrollbar grow overflow-auto">
        {backups === undefined ? (
          <Loading />
        ) : (
          <BackupListForDeployment
            backups={backups}
            selectedDeployment={selectedDeployment}
            targetDeployment={targetDeployment}
            restoringBackupId={restoringBackupId}
            team={team}
            canPerformActions={canPerformActions}
            maxCloudBackups={maxCloudBackups}
          />
        )}
      </div>
    </div>
  );
}

function BackupListForDeployment({
  backups,
  selectedDeployment,
  targetDeployment,
  restoringBackupId,
  team,
  canPerformActions,
  maxCloudBackups,
}: {
  backups: BackupResponse[];
  selectedDeployment: DeploymentResponse;
  targetDeployment: DeploymentResponse;
  restoringBackupId: bigint | null;
  team: TeamResponse;
  canPerformActions: boolean;
  maxCloudBackups: number;
}) {
  const existingCloudBackup = useQuery(udfs.latestExport.latestCloudExport);

  const selectedDeploymentBackups = backups.filter(
    (b) => b.sourceDeploymentId === selectedDeployment.id,
  );

  const latestBackupInTargetDeployment =
    backups?.find((s) => s.sourceDeploymentId === targetDeployment.id) ?? null;

  const someBackupInProgress = backups.some(
    (backup) =>
      backup.sourceDeploymentId === targetDeployment.id &&
      (backup.state === "requested" || backup.state === "inProgress"),
  );

  const getZipExportUrl = useGetZipExport({
    format: "zip",
    include_storage: true,
  });

  return selectedDeploymentBackups.length === 0 ? (
    <EmptySection
      Icon={ArchiveIcon}
      header="No backups in this deployment."
      body="With backups, you can periodically generate snapshots of your deployment data to restore later."
      learnMoreButton={{
        href: "https://docs.convex.dev/database/backup-restore",
        children: "Learn more about backups.",
      }}
      sheet={false}
    />
  ) : (
    <div className="flex flex-col divide-y px-4 py-2">
      {selectedDeploymentBackups.map((backup) => (
        <BackupListItem
          key={backup.id}
          backup={backup}
          restoring={BigInt(backup.id) === restoringBackupId}
          someBackupInProgress={someBackupInProgress}
          someRestoreInProgress={restoringBackupId !== null}
          latestBackupInTargetDeployment={latestBackupInTargetDeployment}
          targetDeployment={targetDeployment}
          team={team}
          getZipExportUrl={getZipExportUrl}
          canPerformActions={canPerformActions}
          maxCloudBackups={maxCloudBackups}
          progressMessage={progressMessageForBackup(
            backup,
            existingCloudBackup,
          )}
        />
      ))}
    </div>
  );
}
