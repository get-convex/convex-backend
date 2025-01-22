import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { AuthConfig } from "components/deploymentSettings/AuthConfig";

export { getServerSideProps } from "lib/ssr";

function AuthConfigurationSettingsPage() {
  return (
    <DeploymentSettingsLayout page="authentication">
      <AuthConfig />
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(AuthConfigurationSettingsPage);
