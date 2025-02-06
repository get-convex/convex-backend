import { Sheet } from "dashboard-common/elements/Sheet";
import {
  DeploymentUrl,
  HttpActionsUrl,
} from "dashboard-common/features/settings/components/DeploymentUrl";
import { DeploymentSettingsLayout } from "dashboard-common/layouts/DeploymentSettingsLayout";

export default function Settings() {
  return (
    <DeploymentSettingsLayout page="url-and-deploy-key">
      <Sheet>
        <DeploymentUrl>
          Configure a production Convex client with this URL.
        </DeploymentUrl>
      </Sheet>
      <Sheet>
        <HttpActionsUrl />
      </Sheet>
      <Sheet>
        <div className="text-content-primary">
          <h4 className="mb-4">Deploy Key</h4>

          <p className="text-content-secondary">
            Deploy keys are not available for self-hosted deployments.
          </p>
        </div>
      </Sheet>
    </DeploymentSettingsLayout>
  );
}
