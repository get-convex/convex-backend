import { DeploymentSettingsLayout } from "../../../layouts/DeploymentSettingsLayout";
import { DeploymentEnvironmentVariables } from "./DeploymentEnvironmentVariables";

export function EnvironmentVariablesView() {
  return (
    <DeploymentSettingsLayout page="environment-variables">
      <DeploymentEnvironmentVariables />
    </DeploymentSettingsLayout>
  );
}
