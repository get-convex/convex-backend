import { BackupResponse } from "api/backups";
import { useDeploymentById } from "api/deployments";
import { useCurrentTeam } from "api/teams";
import { useProjectById } from "api/projects";

export function BackupIdentifier({ backup }: { backup: BackupResponse }) {
  const team = useCurrentTeam();
  const deployment = useDeploymentById(
    team?.id || 0,
    backup.sourceDeploymentId,
  );
  const project = useProjectById(team?.id, deployment?.projectId);
  return (
    <span className="text-xs text-content-secondary">
      {team?.slug}-{project?.slug}-{deployment?.name}-
      {new Date(backup.requestedTime).getTime()}
    </span>
  );
}
