import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { Sheet } from "dashboard-common/elements/Sheet";
import { NonProdDeploymentWarning } from "components/deploymentSettings/NonProdDeploymentWarning";
import { DeployKeysForDeployment } from "components/deploymentSettings/DeployKeysForDeployment";
import { useCurrentDeployment } from "api/deployments";
import { useRouter } from "next/router";
import { usePathname } from "next/navigation";
import { DeletePreviewDeployment } from "components/deploymentSettings/DeletePreviewDeployment";
import { DeploymentSettingsLayout } from "dashboard-common/layouts/DeploymentSettingsLayout";
import {
  DeploymentUrl,
  HttpActionsUrl,
} from "dashboard-common/features/settings/components/DeploymentUrl";

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
    <DeploymentSettingsLayout page="url-and-deploy-key">
      <DeploymentURLAndDeployKey />
    </DeploymentSettingsLayout>
  );
});

function DeploymentURLAndDeployKey() {
  const deployment = useCurrentDeployment();
  const deploymentType = deployment?.deploymentType ?? "prod";

  switch (deploymentType) {
    case "prod":
      return (
        <>
          <Sheet>
            <DeploymentUrl>
              Configure a production Convex client with this URL.
            </DeploymentUrl>
          </Sheet>
          <Sheet>
            <HttpActionsUrl />
          </Sheet>
          <Sheet>
            <DeployKeysForDeployment />
          </Sheet>
        </>
      );
    case "dev":
      return (
        <NonProdDeploymentWarning deploymentType={deploymentType}>
          <div className="flex flex-col gap-4 p-6 pt-0">
            <div>
              <DeploymentUrl>
                Configure a Convex client with this URL while developing
                locally.
              </DeploymentUrl>
            </div>
            <div>
              <HttpActionsUrl />
            </div>
            <div>
              <DeployKeysForDeployment />
            </div>
          </div>
        </NonProdDeploymentWarning>
      );
    case "preview":
      return (
        <div className="flex flex-col gap-4">
          <NonProdDeploymentWarning deploymentType={deploymentType}>
            <div className="flex flex-col gap-4 p-6 pt-0">
              <div>
                <DeploymentUrl>
                  Configure a Convex client with this URL to preview changes on
                  a branch.
                </DeploymentUrl>
              </div>
              <div>
                <HttpActionsUrl />
              </div>
              <div>
                <DeployKeysForDeployment />
              </div>
            </div>
          </NonProdDeploymentWarning>
          <DeletePreviewDeployment />
        </div>
      );
    default: {
      const _typecheck: never = deploymentType;
      return null;
    }
  }
}
