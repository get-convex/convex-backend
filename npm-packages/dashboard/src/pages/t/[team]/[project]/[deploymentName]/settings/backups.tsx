import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { Backups } from "components/deploymentSettings/Backups";
import { useCurrentDeployment } from "api/deployments";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import { Loading } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

function BackupPage() {
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const entitlements = useTeamEntitlements(team?.id);

  return (
    <DeploymentSettingsLayout page="backups">
      {team && deployment && entitlements ? (
        <div className="h-full animate-fadeInFromLoading">
          <Backups
            team={team}
            deployment={deployment}
            entitlements={entitlements}
          />
        </div>
      ) : (
        <Loading />
      )}
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(BackupPage);
