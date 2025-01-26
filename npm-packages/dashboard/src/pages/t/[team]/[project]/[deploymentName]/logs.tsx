import { LogsView } from "dashboard-common";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function LogsPage() {
  return (
    <>
      <DeploymentPageTitle title="Logs" />
      <LogsView />
    </>
  );
}

export default withAuthenticatedPage(LogsPage);
