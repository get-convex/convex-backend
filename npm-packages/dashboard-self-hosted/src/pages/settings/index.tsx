import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { useRef } from "react";
import { useScrollToHash } from "@common/lib/useScrollToHash";
import { Link } from "@ui/Link";
import { Sheet } from "@ui/Sheet";

import { SelfHostedDeploymentSummary } from "../../components/SelfHostedDeploymentSummary";

export default function Settings() {
  const pauseDeploymentRef = useRef<HTMLDivElement | null>(null);

  useScrollToHash("#pause-deployment", pauseDeploymentRef);

  return (
    <DeploymentSettingsLayout page="general">
      <div className="flex flex-col gap-4">
        <SelfHostedDeploymentSummary />
        <Sheet>
          <h3>Deploy Key</h3>
          <p className="mt-2 max-w-prose text-content-secondary">
            Deploy keys are only available for cloud deployments.
          </p>
          <p className="mt-2 max-w-prose text-content-primary">
            Instead, manage admin keys for this deployment from{" "}
            <Link href="/settings/admin-keys" passHref>
              Admin Keys
            </Link>
            , or generate one from the command line using{" "}
            <Link
              href="https://github.com/get-convex/convex-backend/tree/main/self-hosted#docker-configuration"
              target="_blank"
            >
              the script in your repository
            </Link>
            .
          </p>
        </Sheet>
        <div ref={pauseDeploymentRef}>
          <PauseDeployment />
        </div>
      </div>
    </DeploymentSettingsLayout>
  );
}
