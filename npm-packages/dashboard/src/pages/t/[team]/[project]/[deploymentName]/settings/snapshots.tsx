import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

import { SnapshotExport } from "components/deploymentSettings/SnapshotExport";
import { SnapshotImport } from "components/deploymentSettings/SnapshotImport";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";

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
