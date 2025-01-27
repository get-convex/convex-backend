import { useContext } from "react";
import Link from "next/link";
import { useAdminKey, useDeploymentUrl } from "../../../lib/deploymentApi";
import { useNents } from "../../../lib/useNents";
import { reportHttpError, toast } from "../../../lib/utils";
import { DeploymentInfoContext } from "../../../lib/deploymentContext";
import { displayName } from "../../../lib/functions/generateFileTree";

export function useCancelAllJobs(): (udfPath?: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { selectedNent } = useNents();
  const { deploymentsURI } = useContext(DeploymentInfoContext);

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
              href={`${deploymentsURI}settings/pause-deployment`}
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
