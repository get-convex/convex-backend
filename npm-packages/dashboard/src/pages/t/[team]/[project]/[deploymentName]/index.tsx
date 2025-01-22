import { HealthWithInsights } from "components/health/HealthWithInsights";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { PageContent } from "dashboard-common";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function HealthPage() {
  return (
    <PageContent>
      <DeploymentPageTitle title="Health" />
      <HealthWithInsights />
    </PageContent>
  );
}

export default withAuthenticatedPage(HealthPage);
