import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentSettingsLayout } from "dashboard-common";
import { SnapshotExport } from "components/deploymentSettings/SnapshotExport";
import { SnapshotImport } from "components/deploymentSettings/SnapshotImport";

export { getServerSideProps } from "lib/ssr";

function SnapshotExportPage() {
  return (
    <DeploymentSettingsLayout page="snapshots">
      <SnapshotExport />
      <SnapshotImport />
    </DeploymentSettingsLayout>
  );
}

export default withAuthenticatedPage(SnapshotExportPage);
