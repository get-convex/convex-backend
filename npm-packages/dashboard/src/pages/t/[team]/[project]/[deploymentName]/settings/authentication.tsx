import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { AuthenticationView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(AuthenticationView);
