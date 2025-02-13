import { Sheet } from "dashboard-common/elements/Sheet";
import {
  DeploymentUrl,
  HttpActionsUrl,
} from "dashboard-common/features/settings/components/DeploymentUrl";
import { DeploymentSettingsLayout } from "dashboard-common/layouts/DeploymentSettingsLayout";
import Link from "next/link";

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

          <p className="max-w-prose text-content-secondary">
            Deploy keys are not available for self-hosted deployments.{" "}
          </p>
          <p className="mt-1 max-w-prose text-content-secondary">
            Instead, generate an admin key instead using{" "}
            <Link
              href="https://github.com/get-convex/convex-backend/tree/main/self-hosted#docker-configuration"
              className="text-content-link hover:underline"
            >
              the script in your repository
            </Link>
            .
          </p>
        </div>
      </Sheet>
    </DeploymentSettingsLayout>
  );
}
