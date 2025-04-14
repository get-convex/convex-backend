import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { HistoryView } from "@common/features/history/components/HistoryView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(HistoryView);
