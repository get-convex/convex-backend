import {
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useState,
} from "react";
import { useRouter } from "next/router";
import {
  useAdminKey,
  useDeploymentAuthHeader,
  useDeploymentUrl,
} from "dashboard-common/lib/deploymentApi";
import { toast } from "dashboard-common/lib/utils";
import {
  CompletedExport,
  DatadogSiteLocation,
  IntegrationType,
} from "system-udfs/convex/_system/frontend/common";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { CreateDeploymentAccessTokenRequest } from "api/accessTokens";
import { useCurrentTeam, useTeamEntitlements, useTeamMembers } from "api/teams";
import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import { useCurrentUsageBanner } from "components/header/UsageBanner";
import { useIsDeploymentPaused } from "hooks/useIsDeploymentPaused";
import { CloudImport } from "elements/BackupIdentifier";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { logDeploymentEvent } from "convex-analytics";
import {
  ErrorBoundary,
  captureException,
  captureMessage,
} from "@sentry/nextjs";
import { reportHttpError } from "hooks/fetching";
import { Fallback } from "pages/500";
import {
  ConnectedDeploymentContext,
  DeploymentApiProvider,
  DeploymentApiProviderProps,
  DeploymentInfo,
  DeploymentInfoContext,
} from "dashboard-common/lib/deploymentContext";
import { useAccessToken } from "./useServerSideData";
import { useCurrentProject } from "../api/projects";
import { useTeamUsageState } from "./useTeamUsageState";
import { useProjectEnvironmentVariables } from "./api";

// A silly, standard hack to dodge warnings about useLayoutEffect on the server.
const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

function DeploymentErrorBoundary({ children }: { children: React.ReactNode }) {
  return <ErrorBoundary fallback={Fallback}>{children}</ErrorBoundary>;
}

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
  const teamsURI = `/t/${selectedTeamSlug}`;
  const projectsURI = `${teamsURI}/${projectSlug}`;
  const deploymentsURI = `${projectsURI}/${deploymentName}`;
  useIsomorphicLayoutEffect(() => {
    const f = async () => {
      const info = await deploymentAuth(
        deploymentOverride || (deploymentName as string),
        `Bearer ${accessToken}`,
      );
      setDeploymentInfo({
        ...info,
        captureMessage,
        captureException,
        reportHttpError,
        useCurrentTeam,
        useCurrentProject,
        useCurrentUsageBanner,
        useTeamUsageState,
        useCurrentDeployment,
        useTeamMembers,
        useTeamEntitlements,
        useHasProjectAdminPermissions,
        useProjectEnvironmentVariables,
        useIsDeploymentPaused,
        useLogDeploymentEvent,
        TeamMemberLink,
        CloudImport,
        ErrorBoundary: DeploymentErrorBoundary,
        teamsURI,
        projectsURI,
        deploymentsURI,
        isSelfHosted: false,
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

export function useLogDeploymentEvent() {
  const deployment = useContext(ConnectedDeploymentContext);
  if (!deployment) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();
  return useCallback(
    (msg: string, props: object | null = null) => {
      logDeploymentEvent(msg, deploymentUrl, authHeader, props);
    },
    [deploymentUrl, authHeader],
  );
}
