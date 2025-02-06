import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

import { PauseDeployment } from "components/deploymentSettings/PauseDeployment";
import { DeploymentSettingsLayout } from "dashboard-common/layouts/DeploymentSettingsLayout";

export { getServerSideProps } from "lib/ssr";

function PauseDeploymentPage() {
  return (
    <DeploymentSettingsLayout page="pause-deployment">
      <PauseDeployment />
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(PauseDeploymentPage);
