import { useCurrentDeployment } from "api/deployments";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import {
  useCreateDeployKey,
  useDeleteDeployKey,
  useDeployKeys,
} from "api/accessTokens";
import { useHasProjectAdminPermissions } from "api/roles";
import { Link } from "@ui/Link";

import { DeploymentAccessTokenList } from "./DeploymentAccessTokenList";

export function DeployKeysForDeployment() {
  const project = useCurrentProject();
  const team = useCurrentTeam();
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);

  const deployment = useCurrentDeployment();
  const deploymentType = deployment?.deploymentType ?? "prod";

  const disabledReason =
    deploymentType === "prod" && !hasAdminPermissions
      ? "CannotManageProd"
      : deployment?.kind === "local"
        ? "LocalDeployment"
        : null;

  const createDeployKey = useCreateDeployKey(deployment?.name || "");
  const deleteDeployKey = useDeleteDeployKey(deployment?.name || "");

  const deployKeys = useDeployKeys(
    disabledReason === null ? deployment?.name : undefined,
  );

  const deployKeyDescription = (
    <p className="mb-2 max-w-prose text-content-primary">
      Generate a deploy key to configure Convex integrations, such as
      automatically deploying to a{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting"
        target="_blank"
      >
        hosting provider
      </Link>{" "}
      (like Netlify or Vercel) or syncing data with{" "}
      <Link
        passHref
        href="https://docs.convex.dev/database/import-export/streaming"
        target="_blank"
      >
        Fivetran or Airbyte
      </Link>
      .
    </p>
  );
  return (
    <div className="w-full">
      {team && deployment && (
        <DeploymentAccessTokenList
          header="Deploy Keys"
          buttonProps={{
            deploymentType,
            getAdminKey: async (
              name: string,
              allowedOperations: string[] | undefined,
            ) => {
              try {
                const result = await createDeployKey(
                  // @ts-expect-error allowedOperations is not in the public API spec yet
                  { name, allowedOperations },
                );
                if (!result) return { ok: false as const };
                return { ok: true as const, adminKey: result.deployKey };
              } catch {
                return { ok: false as const };
              }
            },
            disabledReason,
          }}
          description={deployKeyDescription}
          deploymentType={deploymentType}
          onDelete={deleteDeployKey}
          deployKeys={deployKeys}
          disabledReason={disabledReason}
        />
      )}
    </div>
  );
}
