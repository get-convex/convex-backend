import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ComponentsView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(ComponentsView);
