import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { AuthConfig } from "features/settings/components/AuthConfig";

export function AuthenticationView() {
  return (
    <DeploymentSettingsLayout page="authentication">
      <AuthConfig />
    </DeploymentSettingsLayout>
  );
}
