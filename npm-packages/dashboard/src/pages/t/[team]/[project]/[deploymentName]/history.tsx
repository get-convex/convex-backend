import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { History, PageContent } from "dashboard-common";
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
