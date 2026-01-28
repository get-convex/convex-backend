import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { Sheet } from "@ui/Sheet";
import { DeployKeysForDeployment } from "components/deploymentSettings/DeployKeysForDeployment";
import { useCurrentDeployment } from "api/deployments";
import { useRouter } from "next/router";
import { usePathname } from "next/navigation";
import { DeletePreviewDeployment } from "components/deploymentSettings/DeletePreviewDeployment";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import {
  DeploymentUrl,
  HttpActionsUrl,
} from "@common/features/settings/components/DeploymentUrl";
import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { useScrollToHash } from "@common/lib/useScrollToHash";
import { usePostHog } from "hooks/usePostHog";
import { useRef } from "react";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => {
  const router = useRouter();
  const envVars = router.query.var;
  const pathname = usePathname();

  // If "var" is present as a query parameter, we route to settings/environment-variables since, previously,
  // all deployment settings were on the same page and this was handled without routing. We don't want
  // to break links to this so we just manually handle this here.
  if (envVars) {
    void router.push({
      pathname: `${pathname}/environment-variables`,
      query: { var: envVars },
    });
  }

  return (
    <DeploymentSettingsLayout page="general">
      <DeploymentURLAndDeployKey />
    </DeploymentSettingsLayout>
  );
});

function DeploymentURLAndDeployKey() {
  const deployment = useCurrentDeployment();
  const deploymentType = deployment?.deploymentType ?? "prod";
  const { capture } = usePostHog();
  const pauseDeploymentRef = useRef<HTMLDivElement | null>(null);
  useScrollToHash("#pause-deployment", pauseDeploymentRef);

  const getDeploymentUrlDescription = () => {
    switch (deploymentType) {
      case "prod":
        return "Configure a production Convex client with this URL.";
      case "dev":
        return "Configure a Convex client with this URL while developing locally.";
      case "preview":
        return "Configure a Convex client with this URL to preview changes on a branch.";
      case "custom":
        return "Configure a Convex client with this URL.";
      default:
        return "Configure a Convex client with this URL.";
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <Sheet>
        <DeploymentUrl>{getDeploymentUrlDescription()}</DeploymentUrl>
      </Sheet>
      <Sheet>
        <HttpActionsUrl />
      </Sheet>
      <Sheet>
        <DeployKeysForDeployment />
      </Sheet>
      <div ref={pauseDeploymentRef}>
        <PauseDeployment
          onPausedDeployment={() => {
            capture("paused_deployment");
          }}
        />
      </div>
      {deploymentType === "preview" && <DeletePreviewDeployment />}
    </div>
  );
}
