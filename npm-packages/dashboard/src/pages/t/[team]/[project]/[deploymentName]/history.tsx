import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { PageContent } from "dashboard-common";
import { History } from "components/history/History";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function HistoryPage() {
  return (
    <PageContent>
      <DeploymentPageTitle title="History" />
      <History />
    </PageContent>
  );
}

export default withAuthenticatedPage(HistoryPage);
