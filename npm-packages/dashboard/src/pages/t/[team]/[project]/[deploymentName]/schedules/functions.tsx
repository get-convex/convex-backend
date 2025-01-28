import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ScheduledFunctionsView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(ScheduledFunctionsView);
