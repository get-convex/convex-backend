import { useCallback, useContext } from "react";
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
import { logDeploymentEvent } from "convex-analytics";
import { reportHttpError } from "hooks/fetching";
import { ConnectedDeploymentContext } from "dashboard-common/lib/deploymentContext";
import { AuthorizeArgs, AuthorizeResponse } from "generatedApi";

export const deviceTokenDeploymentAuth = async (
  accessTokenArgs: {
    name: string;
    teamId: number;
    deploymentId: number | null;
    projectId: number | null;
    permissions: string[] | null;
  },
  accessToken: string,
  createAccessToken: (body: AuthorizeArgs) => Promise<AuthorizeResponse>,
): Promise<
  | { adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => {
  const data = await createAccessToken({
    authnToken: accessToken,
    deviceName: accessTokenArgs.name,
    teamId: accessTokenArgs.teamId,
    deploymentId: accessTokenArgs.deploymentId,
    projectId: accessTokenArgs.projectId,
    permissions: accessTokenArgs.permissions,
  });

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

export function useCreateSentrySink(): (
  dsn: string,
  tags: Record<string, string>,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (dsn: string, tags: Record<string, string>) => {
    const body = JSON.stringify({ dsn, tags });
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

export const useUpdateCanonicalUrl = (
  requestDestination: "convexCloud" | "convexSite",
) => {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  return useCallback(
    async (url: string | null) => {
      const res = await fetch(`${deploymentUrl}/api/update_canonical_url`, {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          requestDestination,
          url,
        }),
      });
      if (!res.ok) {
        const err = await res.json();
        toast("error", err.message);
      }
    },
    [adminKey, deploymentUrl, requestDestination],
  );
};
