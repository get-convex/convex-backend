import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { FunctionsView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(FunctionsView);
