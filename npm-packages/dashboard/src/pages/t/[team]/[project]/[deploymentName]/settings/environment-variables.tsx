import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { EnvironmentVariablesView } from "@common/features/settings/components/EnvironmentVariablesView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(EnvironmentVariablesView);
