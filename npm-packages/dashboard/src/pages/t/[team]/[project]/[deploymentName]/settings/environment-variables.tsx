import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { DeploymentEnvironmentVariables } from "components/deploymentSettings/DeploymentEnvironmentVariables";

export { getServerSideProps } from "lib/ssr";

function EnvironmentVariablesPage() {
  return (
    <DeploymentSettingsLayout page="environment-variables">
      <DeploymentEnvironmentVariables />
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(EnvironmentVariablesPage);
