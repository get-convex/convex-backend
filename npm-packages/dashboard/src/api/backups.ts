import { components, CloudBackupResponse } from "generatedApi";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { useBBQuery, useBBMutation } from "./api";

// Annotate the backup response with a bit more information.
// TODO: Figure out how to type the enum better on big brain's end.
export type BackupResponse = Omit<CloudBackupResponse, "snapshotId" | "state"> &
  (
    | {
        state: "complete" | "inProgress";
        // document id of the snapshot stored in the backend
        snapshotId: Id<"_exports">;
      }
    | { state: "requested" | "failed" | "canceled" }
  );

export type PeriodicBackupConfig = {
  sourceDeploymentId: number;
  cronspec: string;
  expirationDeltaSecs: number;
  nextRun: number;
};

export function useListCloudBackups(teamId: number) {
  const { data } = useBBQuery({
    path: `/teams/{team_id}/list_cloud_backups`,
    pathParams: {
      team_id: teamId.toString(),
    },
    swrOptions: { refreshInterval: 5000 },
  });
  return data as BackupResponse[] | undefined;
}

export function useGetCloudBackup(cloudBackupId?: number) {
  const { data } = useBBQuery({
    path: `/cloud_backups/{cloud_backup_id}`,
    pathParams: {
      cloud_backup_id: cloudBackupId?.toString() || "",
    },
    swrOptions: {
      refreshInterval: 5000,
    },
  });
  return data as BackupResponse | undefined;
}

export function useRequestCloudBackup(
  deploymentId?: components["schemas"]["DeploymentId"],
  teamId?: number,
) {
  return useBBMutation({
    path: `/deployments/{deployment_id}/request_cloud_backup`,
    pathParams: {
      deployment_id: deploymentId?.toString() || "",
    },
    mutateKey: `/teams/{team_id}/list_cloud_backups`,
    mutatePathParams: {
      team_id: teamId?.toString() || "",
    },
    successToast: "Started a new backup.",
  });
}

export function useDeleteCloudBackup(teamId: number, cloudBackupId?: number) {
  return useBBMutation({
    path: `/cloud_backups/{cloud_backup_id}/delete`,
    pathParams: {
      cloud_backup_id: cloudBackupId?.toString() || "",
    },
    mutateKey: `/teams/{team_id}/list_cloud_backups`,
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "Backup deleted.",
  });
}

export function useCancelCloudBackup(teamId: number, cloudBackupId?: number) {
  return useBBMutation({
    path: `/cloud_backups/{cloud_backup_id}/cancel`,
    pathParams: {
      cloud_backup_id: cloudBackupId?.toString() || "",
    },
    mutateKey: `/teams/{team_id}/list_cloud_backups`,
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "Backup canceled.",
  });
}

export function useConfigurePeriodicBackup(deploymentId?: number) {
  return useBBMutation({
    path: `/deployments/{deployment_id}/configure_periodic_backup`,
    pathParams: {
      deployment_id: deploymentId?.toString() || "",
    },
    mutateKey: `/deployments/{deployment_id}/get_periodic_backup_config`,
    mutatePathParams: {
      deployment_id: deploymentId?.toString() || "",
    },
  });
}

export function useDisablePeriodicBackup(deploymentId?: number) {
  return useBBMutation({
    path: `/deployments/{deployment_id}/disable_periodic_backup`,
    pathParams: {
      deployment_id: deploymentId?.toString() || "",
    },
    mutateKey: `/deployments/{deployment_id}/get_periodic_backup_config`,
    mutatePathParams: {
      deployment_id: deploymentId?.toString() || "",
    },
    successToast: "Automatic backups are now disabled.",
  });
}

export function useGetPeriodicBackupConfig(deploymentId: number) {
  const { data } = useBBQuery({
    path: `/deployments/{deployment_id}/get_periodic_backup_config`,
    pathParams: {
      deployment_id: deploymentId.toString(),
    },
    swrOptions: { refreshInterval: 5000 },
  });
  return data;
}

export function useRestoreFromCloudBackup(deploymentId?: number) {
  return useBBMutation({
    path: `/deployments/{deployment_id}/restore_from_cloud_backup`,
    pathParams: {
      deployment_id: deploymentId?.toString() || "",
    },
    successToast: "Cloud backup restoration successfully requested.",
  });
}
