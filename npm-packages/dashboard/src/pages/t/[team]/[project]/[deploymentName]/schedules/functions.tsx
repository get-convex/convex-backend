import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { ScheduledFunctionsView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

function FunctionsPage() {
  return (
    <>
      <DeploymentPageTitle title="Scheduled Functions" />
      <ScheduledFunctionsView />
    </>
  );
}

export default withAuthenticatedPage(FunctionsPage);
