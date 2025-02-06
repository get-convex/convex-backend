import { CronsView } from "dashboard-common/features/schedules/components/crons/CronsView";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(CronsView);
