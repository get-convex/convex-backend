import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { Loading } from "@ui/Loading";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { CustomDomains } from "components/deploymentSettings/CustomDomains";
import { useCurrentDeployment } from "api/deployments";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";

export { getServerSideProps } from "lib/ssr";

function CustomDomainsPage() {
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const entitlements = useTeamEntitlements(team?.id);

  return (
    <DeploymentSettingsLayout page="custom-domains">
      {team && deployment && entitlements ? (
        <div className="h-full animate-fadeInFromLoading">
          <CustomDomains
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

export default withAuthenticatedPage(CustomDomainsPage);
