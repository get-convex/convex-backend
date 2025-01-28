import { HealthWithInsights } from "components/health/HealthWithInsights";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(HealthWithInsights);
