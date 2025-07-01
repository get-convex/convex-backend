import { useContext } from "react";
import {
  DeploymentInfo,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { toast } from "@common/lib/utils";
import {
  DatadogSiteLocation,
  IntegrationType,
} from "system-udfs/convex/_system/frontend/common";
import { useAdminKey, useDeploymentUrl } from "./deploymentApi";

export function useCreateDatadogIntegration(): (
  siteLocation: DatadogSiteLocation,
  ddApiKey: string,
  ddTags: string[],
  service: string | null,
  version: "1" | "2",
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

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
    await createIntegration(
      "datadog",
      body,
      deploymentUrl,
      adminKey,
      reportHttpError,
    );
  };
}

export function useCreateWebhookIntegration(): (url: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (url: string) => {
    const body = JSON.stringify({ url });
    await createIntegration(
      "webhook",
      body,
      deploymentUrl,
      adminKey,
      reportHttpError,
    );
  };
}

export function useCreateAxiomIntegration(): (
  datasetName: string,
  apiKey: string,
  attributes: { key: string; value: string }[],
  version: "1" | "2",
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (
    datasetName: string,
    apiKey: string,
    attributes: { key: string; value: string }[],
    version: "1" | "2",
  ) => {
    const body = JSON.stringify({ datasetName, apiKey, attributes, version });
    await createIntegration(
      "axiom",
      body,
      deploymentUrl,
      adminKey,
      reportHttpError,
    );
  };
}

export function useCreateSentryIntegration(): (
  dsn: string,
  tags: Record<string, string>,
  version: "1" | "2",
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (
    dsn: string,
    tags: Record<string, string>,
    version: "1" | "2",
  ) => {
    const body = JSON.stringify({ dsn, tags, version });
    await createIntegration(
      "sentry",
      body,
      deploymentUrl,
      adminKey,
      reportHttpError,
    );
  };
}

async function createIntegration(
  integrationType: IntegrationType,
  body: string,
  deploymentUrl: string,
  adminKey: string,
  reportHttpError: DeploymentInfo["reportHttpError"],
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

export function useDeleteIntegration(): (
  integrationType: IntegrationType,
) => Promise<void> {
  const { reportHttpError } = useContext(DeploymentInfoContext);
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
