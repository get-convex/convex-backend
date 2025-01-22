import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { SnapshotExport } from "components/deploymentSettings/SnapshotExport";

export { getServerSideProps } from "lib/ssr";

function SnapshotExportPage() {
  return (
    <DeploymentSettingsLayout page="snapshots">
      <SnapshotExport />
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(SnapshotExportPage);
