import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { PauseDeployment } from "components/deploymentSettings/PauseDeployment";

export { getServerSideProps } from "lib/ssr";

function PauseDeploymentPage() {
  return (
    <DeploymentSettingsLayout page="pause-deployment">
      <PauseDeployment />
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(PauseDeploymentPage);
