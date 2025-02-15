import { deviceTokenDeploymentAuth } from "hooks/deploymentApi";
import { useCurrentDeployment } from "api/deployments";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import {
  useCreateTeamAccessToken,
  useInstanceAccessTokens,
} from "api/accessTokens";
import { useHasProjectAdminPermissions } from "api/roles";
import Link from "next/link";
import {
  DeploymentResponse,
  Team,
  ProjectDetails,
  AuthorizeArgs,
  AuthorizeResponse,
} from "generatedApi";

import { useAccessToken } from "hooks/useServerSideData";
import { DeploymentType } from "dashboard-common/features/settings/components/DeploymentUrl";
import { DeploymentAccessTokenList } from "./DeploymentAccessTokenList";

function getAdminKeyPrefix(deployment: DeploymentResponse) {
  switch (deployment.deploymentType) {
    case "prod":
      return "prod";
    case "dev":
      return "dev";
    case "preview": {
      return "preview";
    }
    default: {
      const _typecheck: never = deployment.deploymentType;
      return "";
    }
  }
}

function toDeployKeyResponse(
  prefix: string,
  accessTokenBasedDeployKey:
    | { adminKey: string; ok: true }
    | { ok: false; errorMessage: string; errorCode: string },
): { ok: true; adminKey: string } | { ok: false } {
  return accessTokenBasedDeployKey.ok
    ? {
        ok: true,
        adminKey: `${prefix}|${accessTokenBasedDeployKey.adminKey}`,
      }
    : {
        ok: false,
      };
}

export async function getAccessTokenBasedDeployKey(
  deployment: DeploymentResponse,
  project: ProjectDetails | undefined,
  team: Team,
  prefix: string,
  accessToken: string,
  createAccessTokenMutation: (
    body: AuthorizeArgs,
  ) => Promise<AuthorizeResponse>,
  tokenName?: string,
): Promise<{ ok: true; adminKey: string } | { ok: false }> {
  let environmentDisplayName = "";
  if (deployment.deploymentType === "preview") {
    environmentDisplayName = "Preview";
  } else if (deployment.deploymentType === "dev") {
    environmentDisplayName = "Development";
  } else if (deployment.deploymentType === "prod") {
    environmentDisplayName = "Production";
  } else {
    environmentDisplayName = deployment.deploymentType;
  }

  const name = tokenName || `${project?.slug}: ${environmentDisplayName}`;
  const accessTokenBasedDeployKey = await deviceTokenDeploymentAuth(
    {
      name,
      teamId: team?.id || 0,
      deploymentId: deployment?.id || 0,
      projectId: null,
      permissions: null,
    },
    accessToken,
    createAccessTokenMutation,
  );

  return toDeployKeyResponse(prefix, accessTokenBasedDeployKey);
}

export async function getAccessTokenBasedDeployKeyForPreview(
  project: ProjectDetails,
  team: Team,
  prefix: string,
  accessToken: string,
  createAccessTokenMutation: (
    body: AuthorizeArgs,
  ) => Promise<AuthorizeResponse>,
  tokenName?: string,
): Promise<{ ok: true; adminKey: string } | { ok: false }> {
  const accessTokenBasedDeployKey = await deviceTokenDeploymentAuth(
    {
      name: tokenName || `${project.slug}: Preview`,
      teamId: team.id,
      deploymentId: null,
      projectId: project.id,
      permissions: ["preview:*"],
    },
    accessToken,
    createAccessTokenMutation,
  );
  return toDeployKeyResponse(prefix, accessTokenBasedDeployKey);
}

export function DeployKeysForDeployment() {
  const project = useCurrentProject();
  const team = useCurrentTeam();
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);

  const deployment = useCurrentDeployment();
  const deploymentType = deployment?.deploymentType ?? "prod";
  const [accessToken] = useAccessToken();

  const disabledReason =
    deploymentType === "prod" && !hasAdminPermissions
      ? "CannotManageProd"
      : deployment?.kind === "local"
        ? "LocalDeployment"
        : null;

  const createAccessTokenMutation = useCreateTeamAccessToken({
    deploymentName: deployment?.name || "",
    kind: "deployment",
  });

  const accessTokens = useInstanceAccessTokens(
    disabledReason === null ? deployment?.name : undefined,
  );

  const deployKeyDescription =
    deploymentType === "prod" ? (
      <p className="mb-2 text-content-primary">
        Generate and copy this key to configure Convex integrations, such as
        automatically deploying to a{" "}
        <Link
          passHref
          href="https://docs.convex.dev/production/hosting"
          className="text-content-link"
          target="_blank"
        >
          hosting provider
        </Link>{" "}
        (like Netlify or Vercel) or syncing data with{" "}
        <Link
          passHref
          href="https://docs.convex.dev/database/import-export/streaming"
          className="text-content-link"
          target="_blank"
        >
          Fivetran or Airbyte
        </Link>
        .
      </p>
    ) : (
      <p className="mb-2 text-content-primary">
        It's rare to need a <DeploymentType deploymentType={deploymentType} />{" "}
        deploy key.
      </p>
    );
  return (
    <div className="w-full">
      {team && deployment && (
        <DeploymentAccessTokenList
          header="Deploy Keys"
          buttonProps={{
            deploymentType,
            getAdminKey: (name: string) =>
              getAccessTokenBasedDeployKey(
                deployment,
                project,
                team,
                `${getAdminKeyPrefix(deployment)}:${deployment.name}`,
                accessToken,
                createAccessTokenMutation,
                name,
              ),

            disabledReason,
          }}
          description={deployKeyDescription}
          identifier={deployment?.name}
          tokenPrefix={`${deploymentType}:${deployment.name}`}
          accessTokens={accessTokens}
          kind="deployment"
          disabledReason={disabledReason}
        />
      )}
    </div>
  );
}
