import { DeploymentSettingsLayout } from "../../../layouts/DeploymentSettingsLayout";
import { AuthConfig } from "./AuthConfig";

export function AuthenticationView() {
  return (
    <DeploymentSettingsLayout page="authentication">
      <AuthConfig />
    </DeploymentSettingsLayout>
  );
}
