import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { FunctionsView } from "dashboard-common/features/functions/components/FunctionsView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(FunctionsView);
