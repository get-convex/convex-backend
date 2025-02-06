import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DataView } from "dashboard-common/features/data/components/DataView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(DataView);
