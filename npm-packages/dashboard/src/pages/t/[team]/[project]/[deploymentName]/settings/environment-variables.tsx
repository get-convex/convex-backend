import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { EnvironmentVariablesView } from "dashboard-common/features/settings/components/EnvironmentVariablesView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(EnvironmentVariablesView);
