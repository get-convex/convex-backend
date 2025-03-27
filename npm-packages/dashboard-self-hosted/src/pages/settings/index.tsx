import { Sheet } from "dashboard-common/elements/Sheet";
import {
  DeploymentUrl,
  HttpActionsUrl,
} from "dashboard-common/features/settings/components/DeploymentUrl";
import { DeploymentSettingsLayout } from "dashboard-common/layouts/DeploymentSettingsLayout";
import Link from "next/link";
import { useContext } from "react";
import { DeploymentInfoContext } from "dashboard-common/lib/deploymentContext";
import { CopyTextButton } from "dashboard-common/elements/CopyTextButton";

export default function Settings() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const isTryItOutDeployment = deployment?.name?.startsWith("tryitout-");

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
        <div className="flex flex-col gap-2 text-content-primary">
          <h4 className="mb-4">Deploy Key</h4>

          <p className="max-w-prose text-content-secondary">
            Deploy keys are only available for cloud deployments.
          </p>
          {isTryItOutDeployment ? (
            <>
              <p className="max-w-prose text-content-primary">
                You can create a Convex account and automatically link this
                deployment by running this from your terminal:
              </p>

              <CopyTextButton className="text-sm" text="npx convex login" />
              <Link
                href="https://docs.convex.dev/production/hosting/"
                target="_blank"
                className="text-content-link hover:underline"
              >
                Learn more
              </Link>
            </>
          ) : (
            <p className="mt-1 max-w-prose text-content-primary">
              Instead, generate an admin key instead using{" "}
              <Link
                href="https://github.com/get-convex/convex-backend/tree/main/self-hosted#docker-configuration"
                className="text-content-link hover:underline"
              >
                the script in your repository
              </Link>
              .
            </p>
          )}
        </div>
      </Sheet>
    </DeploymentSettingsLayout>
  );
}
