import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { AuthenticationView } from "dashboard-common/features/settings/components/AuthenticationView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(AuthenticationView);
