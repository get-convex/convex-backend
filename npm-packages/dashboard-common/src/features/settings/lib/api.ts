import { useCallback, useContext } from "react";
import { Id } from "system-udfs/convex/_generated/dataModel";
import {
  useAdminKey,
  useDeploymentAuthHeader,
  useDeploymentUrl,
} from "../../../lib/deploymentApi";
import { reportHttpError, toast } from "../../../lib/utils";
import { ConnectedDeploymentContext } from "../../../lib/deploymentContext";

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
