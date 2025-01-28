import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { EnvironmentVariablesView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(EnvironmentVariablesView);
