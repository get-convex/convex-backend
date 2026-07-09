import { useCallback, useContext } from "react";
import useSWR, { useSWRConfig } from "swr";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { createDeploymentClient } from "@convex-dev/platform";
import { useAdminKey, useDeploymentUrl } from "@common/lib/deploymentApi";
import { toast } from "@common/lib/utils";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import type {
  UsageLimit,
  UsageLimitConfig,
} from "@common/features/settings/components/UsageLimits";

export function useUpdateEnvVars(): (
  changes: {
    name: string;
    value: string | null;
  }[],
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
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

export function useDeleteComponent() {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
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
    [deploymentUrl, adminKey, reportHttpError],
  );
}

export function usePauseDeployment(): () => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
  return async () => {
    const client = createDeploymentClient(deploymentUrl, adminKey);
    const { response } = await client.POST("/pause_deployment");
    if (response.status !== 200) {
      const err = await response.json();
      reportHttpError("POST", response.url, err);
      toast("error", err.message);
    } else {
      toast("success", "Deployment is now paused");
    }
  };
}

export function useUnpauseDeployment(): () => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
  return async () => {
    const client = createDeploymentClient(deploymentUrl, adminKey);
    const { response } = await client.POST("/unpause_deployment");
    if (response.status !== 200) {
      const err = await response.json();
      reportHttpError("POST", response.url, err);
      toast("error", err.message);
    } else {
      toast("success", "Deployment is now running");
    }
  };
}

// --- Usage limits ---

// Shape returned by the usage limits API (UsageLimitConfigResponse). The enum
// fields are typed loosely by the generated client and narrowed in toUsageLimit.
type UsageLimitConfigResponse = {
  id: string;
  name?: string | null;
  metric: string;
  window: string;
  limitType: string;
  limit: number;
  enabled: boolean;
};

function toUsageLimit(response: UsageLimitConfigResponse): UsageLimit {
  return {
    id: response.id,
    name: response.name,
    metric: response.metric as UsageLimit["metric"],
    window: response.window as UsageLimit["window"],
    limitType: response.limitType as UsageLimit["limitType"],
    limit: response.limit,
    enabled: response.enabled,
  };
}

// Normalize an openapi-fetch error body into the { code, message } shape that
// reportHttpError / toast expect.
function normalizeUsageLimitError(error: unknown): {
  code: string;
  message: string;
} {
  if (error && typeof error === "object") {
    const { code, message } = error as { code?: unknown; message?: unknown };
    return {
      code: typeof code === "string" ? code : "UnknownError",
      message: typeof message === "string" ? message : "Something went wrong.",
    };
  }
  return { code: "UnknownError", message: "Something went wrong." };
}

// SWR cache key for the deployment's usage limits list. The read hook and the
// mutation hooks share it so a successful write revalidates the list.
function usageLimitsKey(deploymentUrl: string) {
  return ["usageLimits", deploymentUrl] as const;
}

// Fetch the deployment's usage limits. Backed by SWR so the list is cached,
// deduped, and revalidated after any mutation. Must be used inside a connected
// deployment context.
export function useUsageLimits(): {
  usageLimits: UsageLimit[] | undefined;
  isLoading: boolean;
} {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
  const { data, isLoading } = useSWR(
    usageLimitsKey(deploymentUrl),
    async () => {
      const client = createDeploymentClient(deploymentUrl, adminKey);
      const {
        data: body,
        error,
        response,
      } = await client.GET("/list_usage_limits");
      if (error || !response.ok || !body) {
        const normalized = normalizeUsageLimitError(error);
        reportHttpError("GET", response.url, normalized);
        toast("error", normalized.message);
        throw new Error(normalized.message);
      }
      return body.usageLimits.map(toUsageLimit);
    },
  );
  return { usageLimits: data, isLoading };
}

export function useCreateUsageLimit(): (
  config: UsageLimitConfig,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
  const { mutate } = useSWRConfig();
  return useCallback(
    async (config) => {
      const client = createDeploymentClient(deploymentUrl, adminKey);
      const { error, response } = await client.POST("/create_usage_limit", {
        body: config,
      });
      if (error || !response.ok) {
        const normalized = normalizeUsageLimitError(error);
        reportHttpError("POST", response.url, normalized);
        toast("error", normalized.message);
        // Throw so the caller can tell the save failed and keep its editor
        // open; the toast above has already surfaced the error to the user.
        throw new Error(normalized.message);
      }
      await mutate(usageLimitsKey(deploymentUrl));
    },
    [deploymentUrl, adminKey, reportHttpError, mutate],
  );
}

export function useUpdateUsageLimit(): (
  id: string,
  config: UsageLimitConfig,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
  const { mutate } = useSWRConfig();
  return useCallback(
    async (id, config) => {
      const client = createDeploymentClient(deploymentUrl, adminKey);
      const { error, response } = await client.POST(
        "/update_usage_limit/{id}",
        {
          params: { path: { id } },
          body: config,
        },
      );
      if (error || !response.ok) {
        const normalized = normalizeUsageLimitError(error);
        reportHttpError("POST", response.url, normalized);
        toast("error", normalized.message);
        // Throw so the caller can tell the save failed and keep its editor
        // open; the toast above has already surfaced the error to the user.
        throw new Error(normalized.message);
      }
      await mutate(usageLimitsKey(deploymentUrl));
    },
    [deploymentUrl, adminKey, reportHttpError, mutate],
  );
}

export function useDeleteUsageLimit(): (id: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);
  const { mutate } = useSWRConfig();
  return useCallback(
    async (id) => {
      const client = createDeploymentClient(deploymentUrl, adminKey);
      const { error, response } = await client.POST(
        "/delete_usage_limit/{id}",
        {
          params: { path: { id } },
        },
      );
      if (error || !response.ok) {
        const normalized = normalizeUsageLimitError(error);
        reportHttpError("POST", response.url, normalized);
        toast("error", normalized.message);
        return;
      }
      await mutate(usageLimitsKey(deploymentUrl));
    },
    [deploymentUrl, adminKey, reportHttpError, mutate],
  );
}
