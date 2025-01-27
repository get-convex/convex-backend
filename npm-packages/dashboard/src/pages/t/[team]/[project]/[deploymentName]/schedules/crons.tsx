import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { CronsView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

function CronsPage() {
  return (
    <>
      <DeploymentPageTitle title="Cron Jobs" />
      <CronsView />
    </>
  );
}

export default withAuthenticatedPage(CronsPage);
