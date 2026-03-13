import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { Sheet } from "@ui/Sheet";
import { DeployKeysForDeployment } from "components/deploymentSettings/DeployKeysForDeployment";
import { useCurrentDeployment, useDeploymentRegions } from "api/deployments";
import { useRouter } from "next/router";
import { usePathname } from "next/navigation";
import { DeleteDeployment } from "components/deploymentSettings/DeleteDeployment";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentAdvancedSettings } from "components/deploymentSettings/DeploymentAdvancedSettings";
import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { DeploymentSummary } from "@common/features/health/components/DeploymentSummary";
import { useScrollToHash } from "@common/lib/useScrollToHash";
import { usePostHog } from "hooks/usePostHog";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useCurrentTeam, useTeamMembers } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useListCloudBackups } from "api/backups";
import { useMemo, useRef } from "react";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => {
  const router = useRouter();
  const envVars = router.query.var;
  const pathname = usePathname();

  // If "var" is present as a query parameter, we route to settings/environment-variables since, previously,
  // all deployment settings were on the same page and this was handled without routing. We don't want
  // to break links to this so we just manually handle this here.
  if (envVars) {
    void router.push({
      pathname: `${pathname}/environment-variables`,
      query: { var: envVars },
    });
  }

  return (
    <DeploymentSettingsLayout page="general">
      <DeploymentURLAndDeployKey />
    </DeploymentSettingsLayout>
  );
});

function DeploymentURLAndDeployKey() {
  const deployment = useCurrentDeployment();
  const { capture } = usePostHog();
  const pauseDeploymentRef = useRef<HTMLDivElement | null>(null);
  useScrollToHash("#pause-deployment", pauseDeploymentRef);
  const { showReferences } = useLaunchDarkly();

  const team = useCurrentTeam();
  const project = useCurrentProject();
  const backups = useListCloudBackups(team?.id || 0);
  const teamMembers = useTeamMembers(team?.id);
  const { regions } = useDeploymentRegions(team?.id);

  const lastBackupTime = useMemo(() => {
    if (!backups || !deployment || deployment.kind !== "cloud") {
      return undefined;
    }
    const deploymentsBackups = backups.filter(
      (b) => b.sourceDeploymentId === deployment.id && b.state === "complete",
    );
    return deploymentsBackups.length > 0
      ? deploymentsBackups[0].requestedTime
      : null;
  }, [backups, deployment]);

  return (
    <div className="flex flex-col gap-4">
      {deployment && team?.slug && project?.slug && (
        <DeploymentSummary
          deployment={deployment}
          teamSlug={team.slug}
          projectSlug={project.slug}
          lastBackupTime={lastBackupTime}
          teamMembers={teamMembers}
          regions={regions}
        />
      )}
      <Sheet>
        <DeployKeysForDeployment />
      </Sheet>
      {showReferences && <DeploymentAdvancedSettings />}
      <div ref={pauseDeploymentRef}>
        <PauseDeployment
          onPausedDeployment={() => {
            capture("paused_deployment");
          }}
        />
      </div>
      <DeleteDeployment />
    </div>
  );
}
