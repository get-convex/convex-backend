import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { PageContent } from "dashboard-common";
import { FileStorageContent } from "components/storage/FileStorageContent";
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
