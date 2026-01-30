import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ProvisionDeploymentPage } from "components/provisionDeployment/ProvisionDeploymentPage";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => (
  <ProvisionDeploymentPage deploymentType="dev" />
));
