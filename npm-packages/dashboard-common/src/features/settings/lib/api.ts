import { useCallback, useContext } from "react";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { createDeploymentClient } from "@convex-dev/platform";
import { useAdminKey, useDeploymentUrl } from "@common/lib/deploymentApi";
import { toast } from "@common/lib/utils";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

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
