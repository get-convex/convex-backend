import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ScheduledFunctionsView } from "@common/features/schedules/components/ScheduledFunctionsView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(ScheduledFunctionsView);
