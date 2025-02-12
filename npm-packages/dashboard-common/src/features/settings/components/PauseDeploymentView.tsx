import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";

export function PauseDeploymentView() {
  return (
    <DeploymentSettingsLayout page="pause-deployment">
      <PauseDeployment />
    </DeploymentSettingsLayout>
  );
}
