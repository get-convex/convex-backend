import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ProvisionProductionDeploymentPage } from "components/productionProvision/ProvisionProductionDeploymentPage";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(ProvisionProductionDeploymentPage);
