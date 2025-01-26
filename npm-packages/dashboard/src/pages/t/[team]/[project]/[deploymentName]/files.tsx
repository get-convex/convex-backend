import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { PageContent, FileStorageContent } from "dashboard-common";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function FileStorageDataView() {
  return (
    <PageContent>
      <DeploymentPageTitle title="Files" />
      <FileStorageContent />
    </PageContent>
  );
});
