import { ConvexReactClient } from "convex/react";
import { ConvexHttpClient } from "convex/browser";
import { reportHttpError } from "lib/utils";
import {
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useState,
} from "react";
import { useRouter } from "next/router";
import {
  displayName,
  DeploymentApiProviderProps,
  DeploymentApiProvider,
  ConnectedDeploymentContext,
  DeploymentInfoContext,
  DeploymentInfo,
  useNents,
  useAdminKey,
  useDeploymentAuthHeader,
  useDeploymentUrl,
  toast,
} from "dashboard-common";
import {
  CompletedExport,
  DatadogSiteLocation,
  IntegrationType,
} from "system-udfs/convex/_system/frontend/common";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { CreateDeploymentAccessTokenRequest } from "api/accessTokens";
import Link from "next/link";
import { useCurrentTeam, useTeamEntitlements, useTeamMembers } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import { useCurrentUsageBanner } from "components/header/UsageBanner";
import { useIsDeploymentPaused } from "hooks/useIsDeploymentPaused";
import { CloudImport } from "elements/BackupIdentifier";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { useAccessToken } from "./useServerSideData";

// A silly, standard hack to dodge warnings about useLayoutEffect on the server.
const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

export function DeploymentInfoProvider({
  children,
  deploymentOverride,
}: {
  children: React.ReactNode;
  deploymentOverride?: string;
}): JSX.Element {
  const router = useRouter();
  const { deploymentName } = router.query;
  const [deploymentInfo, setDeploymentInfo] = useState<
    DeploymentInfo | undefined
  >(undefined);

  const [accessToken] = useAccessToken();
  const selectedTeamSlug = router.query.team as string;
  const projectSlug = router.query.project as string;
  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  const deploymentsURI = `${projectsURI}/${deploymentName}/`;
  useIsomorphicLayoutEffect(() => {
    const f = async () => {
      const info = await deploymentAuth(
        deploymentOverride || (deploymentName as string),
        `Bearer ${accessToken}`,
      );
      setDeploymentInfo({
        ...info,
        useCurrentTeam,
        useCurrentUsageBanner,
        useCurrentDeployment,
        useTeamMembers,
        useTeamEntitlements,
        useHasProjectAdminPermissions,
        useIsDeploymentPaused,
        TeamMemberLink,
        CloudImport,
        projectsURI,
        deploymentsURI,
      });
    };
    if (accessToken && (deploymentOverride || deploymentName)) {
      void f();
    }
  }, [accessToken, deploymentName, deploymentOverride]);

  return deploymentInfo ? (
    <DeploymentInfoContext.Provider value={deploymentInfo}>
      {children}
    </DeploymentInfoContext.Provider>
  ) : (
    <>{children}</>
  );
}

const deploymentAuthInner = async (
  deploymentName: string,
  authHeader: string,
  authMethod: string,
): Promise<
  | { deploymentUrl: string; adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => {
  const resp = await fetch(
    `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/instances/${deploymentName}/${authMethod}`,
    {
      method: "POST",
      headers: { Authorization: authHeader },
    },
  );
  const data = await resp.json();
  if (!resp.ok) {
    return { ok: false, errorCode: data.code, errorMessage: data.message };
  }
  const { adminKey, instanceUrl } = data;
  const deploymentUrl = instanceUrl.endsWith("/")
    ? instanceUrl.slice(0, -1)
    : instanceUrl;
  return { deploymentUrl, adminKey, ok: true };
};

// Obtain a deploy key to be displayed to the user for them to use
// in machine based workflows like CI/CD.
const deploymentAuth = async (
  deploymentName: string,
  authHeader: string,
): Promise<
  | { deploymentUrl: string; adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => deploymentAuthInner(deploymentName, authHeader, "auth");

export const deviceTokenDeploymentAuth = async (
  accessTokenArgs: {
    name: string;
    teamId: number;
    deploymentId: number | null;
    projectId: number | null;
    permissions: string[] | null;
  },
  accessToken: string,
  createAccessToken: (
    body: CreateDeploymentAccessTokenRequest,
  ) => Promise<globalThis.Response>,
): Promise<
  | { adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => {
  const resp = await createAccessToken({
    authnToken: accessToken,
    deviceName: accessTokenArgs.name,
    teamId: accessTokenArgs.teamId,
    deploymentId: accessTokenArgs.deploymentId,
    projectId: accessTokenArgs.projectId,
    permissions: accessTokenArgs.permissions,
  });
  const data = await resp.json();
  if (!resp.ok) {
    return { ok: false, errorCode: data.code, errorMessage: data.message };
  }

  return { adminKey: data.accessToken, ok: true };
};

export function useGetZipExport(
  format: CompletedExport["format"],
): (snapshotId: Id<"_exports">) => string {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  return (snapshotId: Id<"_exports">) => {
    const params = new URLSearchParams({ adminKey });
    if (format?.format === "zip") {
      return `${deploymentUrl}/api/export/zip/${snapshotId}?${params}`;
    }
    throw new Error("expected zip");
  };
}

export function useUpdateEnvVars(): (
  changes: {
    name: string;
    value: string | null;
  }[],
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  return async (changes) => {
    const body = JSON.stringify({ changes });
    const res = await fetch(
      `${deploymentUrl}/api/update_environment_variables`,
      {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Content-Type": "application/json",
        },
        body,
      },
    );
    if (res.status !== 200) {
      const err = await res.json();
      reportHttpError("POST", res.url, err);
      toast("error", err.message);
    }
  };
}

export function useCreateDatadogSink(): (
  siteLocation: DatadogSiteLocation,
  ddApiKey: string,
  ddTags: string[],
  service: string | null,
  version: "1" | "2",
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (
    siteLocation: DatadogSiteLocation,
    ddApiKey: string,
    ddTags: string[],
    service: string | null,
    version: "1" | "2",
  ) => {
    const body = JSON.stringify({
      siteLocation,
      ddApiKey,
      ddTags,
      service,
      version,
    });
    await createSink("datadog", body, deploymentUrl, adminKey);
  };
}

export function useCreateWebhookSink(): (url: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (url: string) => {
    const body = JSON.stringify({ url });
    await createSink("webhook", body, deploymentUrl, adminKey);
  };
}

export function useCreateAxiomSink(): (
  datasetName: string,
  apiKey: string,
  attributes: { key: string; value: string }[],
  version: "1" | "2",
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (
    datasetName: string,
    apiKey: string,
    attributes: { key: string; value: string }[],
    version: "1" | "2",
  ) => {
    const body = JSON.stringify({ datasetName, apiKey, attributes, version });
    await createSink("axiom", body, deploymentUrl, adminKey);
  };
}

export function useCreateSentrySink(): (dsn: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (dsn: string) => {
    const body = JSON.stringify({ dsn });
    await createSink("sentry", body, deploymentUrl, adminKey);
  };
}

async function createSink(
  integrationType: IntegrationType,
  body: string,
  deploymentUrl: string,
  adminKey: string,
): Promise<void> {
  const res = await fetch(`${deploymentUrl}/api/logs/${integrationType}_sink`, {
    method: "POST",
    headers: {
      Authorization: `Convex ${adminKey}`,
      "Content-Type": "application/json",
    },
    body,
  });
  if (res.status !== 200) {
    const err = await res.json();
    reportHttpError("POST", res.url, err);
    toast("error", err.message);
  }
}

export function useDeleteSink(): (
  integrationType: IntegrationType,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (integrationType: IntegrationType) => {
    const body = JSON.stringify({
      sinkType: integrationType,
    });
    const res = await fetch(`${deploymentUrl}/api/logs/delete_sink`, {
      method: "DELETE",
      headers: {
        Authorization: `Convex ${adminKey}`,
        "Content-Type": "application/json",
      },
      body,
    });
    if (res.status !== 200) {
      const err = await res.json();
      reportHttpError("DELETE", res.url, err);
      toast("error", err.message);
    }
  };
}

export function useCancelAllJobs(): (udfPath?: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { selectedNent } = useNents();
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();

  return async (udfPath?: string) => {
    const body = JSON.stringify({
      udfPath,
      componentPath: selectedNent?.path ?? undefined,
      componentId: selectedNent?.id ?? undefined,
    });
    const res = await fetch(`${deploymentUrl}/api/cancel_all_jobs`, {
      method: "POST",
      headers: {
        Authorization: `Convex ${adminKey}`,
        "Content-Type": "application/json",
      },
      body,
    });
    if (res.status !== 200) {
      const err = await res.json();
      reportHttpError("POST", res.url, err);
      if (err.code === "OptimisticConcurrencyControlFailure") {
        toast(
          "error",
          <span>
            There are too many functions being scheduled in this deployment.{" "}
            <Link
              href={`/t/${team?.slug}/${project?.slug}/${deployment?.name}/settings/pause-deployment`}
              className="text-content-link hover:underline dark:underline"
            >
              Pause your deployment
            </Link>{" "}
            to cancel all functions.
          </span>,
          "CancelJobsOCC",
        );
      } else {
        toast("error", err.message);
      }
      throw err;
    } else {
      toast(
        "success",
        udfPath
          ? `Canceled all scheduled runs for ${displayName(udfPath, selectedNent?.path ?? null)}.`
          : "Canceled all scheduled runs.",
      );
    }
  };
}

export function useCancelJob(): (
  id: string,
  componentId: string | null,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (id: string, componentId: string | null) => {
    const body = JSON.stringify({ id, componentId });
    const res = await fetch(`${deploymentUrl}/api/cancel_job`, {
      method: "POST",
      headers: {
        Authorization: `Convex ${adminKey}`,
        "Content-Type": "application/json",
      },
      body,
    });
    if (res.status !== 200) {
      const err = await res.json();
      reportHttpError("POST", res.url, err);
      toast("error", err.message);
    } else {
      toast("success", "Scheduled run canceled.");
    }
  };
}

export async function createConvexAdminClient(
  deploymentName: string,
  authHeader: string,
) {
  const authData = await deploymentAuth(deploymentName, authHeader);
  if (!authData.ok) {
    throw new Error(authData.errorMessage);
  }
  const { deploymentUrl, adminKey } = authData;
  const client = new ConvexReactClient(deploymentUrl, {
    reportDebugInfoToConvex: true,
  });
  client.setAdminAuth(adminKey);
  return { client, adminKey, deploymentUrl };
}

export function MaybeDeploymentApiProvider({
  children,
  deploymentOverride,
}: DeploymentApiProviderProps): JSX.Element {
  const [accessToken] = useAccessToken();
  return accessToken ? (
    <DeploymentApiProvider deploymentOverride={deploymentOverride}>
      {children}
    </DeploymentApiProvider>
  ) : (
    // Render children without the deployment API provider
    // so the page can render and load server-side props.
    // eslint-disable-next-line react/jsx-no-useless-fragment
    <>{children}</>
  );
}

export function useConvexHttpClient(): ConvexHttpClient {
  const { deployment } = useContext(ConnectedDeploymentContext);
  if (!deployment) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  return deployment.httpClient;
}

export function useChangeDeploymentState(): (
  newState: "paused" | "running" | "disabled",
) => Promise<void> {
  const deployment = useContext(ConnectedDeploymentContext);
  if (!deployment) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();
  return async (newState) => {
    const body = JSON.stringify({ newState });
    const res = await fetch(`${deploymentUrl}/api/change_deployment_state`, {
      method: "POST",
      headers: {
        Authorization: authHeader,
        "Content-Type": "application/json",
      },
      body,
    });

    if (res.status !== 200) {
      const err = await res.json();
      reportHttpError("POST", res.url, err);
      toast("error", err.message);
    } else {
      toast("success", `Deployment is now ${newState}`);
    }
  };
}

export function useCancelImport(): (
  id: Id<"_snapshot_imports">,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  return useCallback(
    async (id: Id<"_snapshot_imports">) => {
      const res = await fetch(`${deploymentUrl}/api/cancel_import`, {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ importId: id }),
      });
      if (res.status !== 200) {
        const err = await res.json();
        reportHttpError("DELETE", res.url, err);
        toast("error", err.message);
      }
    },
    [deploymentUrl, adminKey],
  );
}

export function useConfirmImport(): (
  id: Id<"_snapshot_imports">,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  return useCallback(
    async (importId: Id<"_snapshot_imports">) => {
      const url = `${deploymentUrl}/api/perform_import`;
      const res = await fetch(url, {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ importId }),
      });
      if (res.status !== 200) {
        const err = await res.json();
        reportHttpError("DELETE", res.url, err);
        toast("error", err.message);
      }
    },
    [deploymentUrl, adminKey],
  );
}

export function useDeleteComponent() {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  return useCallback(
    async (id: Id<"_components">) => {
      const res = await fetch(`${deploymentUrl}/api/delete_component`, {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ componentId: id }),
      });
      if (res.status !== 200) {
        const err = await res.json();
        reportHttpError("DELETE", res.url, err);
        toast("error", err.message);
      }
    },
    [deploymentUrl, adminKey],
  );
}
