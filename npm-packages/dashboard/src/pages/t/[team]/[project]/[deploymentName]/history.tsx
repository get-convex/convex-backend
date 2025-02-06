import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { HistoryView } from "dashboard-common/features/history/components/HistoryView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(HistoryView);
