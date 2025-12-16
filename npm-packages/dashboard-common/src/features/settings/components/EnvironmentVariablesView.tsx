import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentEnvironmentVariables } from "@common/features/settings/components/DeploymentEnvironmentVariables";

export function EnvironmentVariablesView({
  onEnvironmentVariablesAdded,
}: {
  onEnvironmentVariablesAdded?: (count: number) => void;
}) {
  return (
    <DeploymentSettingsLayout page="environment-variables">
      <DeploymentEnvironmentVariables
        onEnvironmentVariablesAdded={onEnvironmentVariablesAdded}
      />
    </DeploymentSettingsLayout>
  );
}
