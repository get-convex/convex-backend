import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { useNents, LoadingTransition } from "dashboard-common";
import { Components } from "components/deploymentSettings/Components";

export { getServerSideProps } from "lib/ssr";

function ComponentsSettings() {
  const { nents } = useNents();
  return (
    <DeploymentSettingsLayout page="components">
      <LoadingTransition>
        {nents && <Components nents={nents} />}
      </LoadingTransition>
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(ComponentsSettings);
