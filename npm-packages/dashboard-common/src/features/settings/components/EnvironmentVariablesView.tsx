import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentEnvironmentVariables } from "@common/features/settings/components/DeploymentEnvironmentVariables";

export function EnvironmentVariablesView() {
  return (
    <DeploymentSettingsLayout page="environment-variables">
      <DeploymentEnvironmentVariables />
    </DeploymentSettingsLayout>
  );
}
