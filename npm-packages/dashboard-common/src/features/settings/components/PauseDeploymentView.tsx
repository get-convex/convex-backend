import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";

export function PauseDeploymentView({
  onPausedDeployment,
}: {
  onPausedDeployment?: () => void;
}) {
  return (
    <DeploymentSettingsLayout page="pause-deployment">
      <PauseDeployment onPausedDeployment={onPausedDeployment} />
    </DeploymentSettingsLayout>
  );
}
