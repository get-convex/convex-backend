import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { LoadingTransition, useNents, PageContent } from "dashboard-common";
import { Logs } from "components/logs/Logs";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function LogsPage() {
  const { nents, selectedNent } = useNents();
  return (
    <PageContent>
      <DeploymentPageTitle title="Logs" />
      <LoadingTransition>
        {nents && <Logs nents={nents} selectedNent={selectedNent} />}
      </LoadingTransition>
    </PageContent>
  );
}

export default withAuthenticatedPage(LogsPage);
