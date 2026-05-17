import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";

export default function PauseDeploymentPage() {
  return (
    <DeploymentSettingsLayout page="general">
      <PauseDeployment />
    </DeploymentSettingsLayout>
  );
}
