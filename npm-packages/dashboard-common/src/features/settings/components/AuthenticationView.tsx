import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { AuthConfig } from "@common/features/settings/components/AuthConfig";

export function AuthenticationView() {
  return (
    <DeploymentSettingsLayout page="authentication">
      <AuthConfig />
    </DeploymentSettingsLayout>
  );
}
