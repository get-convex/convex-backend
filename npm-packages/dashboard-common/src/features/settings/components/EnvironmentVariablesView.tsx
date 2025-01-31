import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { DeploymentEnvironmentVariables } from "features/settings/components/DeploymentEnvironmentVariables";

export function EnvironmentVariablesView() {
  return (
    <DeploymentSettingsLayout page="environment-variables">
      <DeploymentEnvironmentVariables />
    </DeploymentSettingsLayout>
  );
}
