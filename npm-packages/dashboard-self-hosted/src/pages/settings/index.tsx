import { Sheet } from "@ui/Sheet";
import {
  DeploymentUrl,
  HttpActionsUrl,
} from "@common/features/settings/components/DeploymentUrl";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import Link from "next/link";
import { useContext, useRef } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { useScrollToHash } from "@common/lib/useScrollToHash";

export default function Settings() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const isAnonymousDeployment =
    deployment?.name?.startsWith("anonymous-") ||
    deployment?.name?.startsWith("tryitout-");
  const pauseDeploymentRef = useRef<HTMLDivElement | null>(null);

  useScrollToHash("#pause-deployment", pauseDeploymentRef);

  return (
    <DeploymentSettingsLayout page="general">
      <div className="flex flex-col gap-4">
        <Sheet>
          <DeploymentUrl>
            Configure a Convex client with this URL.
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
            {isAnonymousDeployment ? (
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
        <div ref={pauseDeploymentRef}>
          <PauseDeployment />
        </div>
      </div>
    </DeploymentSettingsLayout>
  );
}
