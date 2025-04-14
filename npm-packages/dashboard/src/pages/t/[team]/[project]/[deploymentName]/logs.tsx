import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { LogsView } from "@common/features/logs/components/LogsView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(LogsView);
