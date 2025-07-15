import { useCallback, useContext } from "react";
import { useSWRConfig } from "swr";
import {
  useDeploymentUrl,
  useAdminKey,
  useDeploymentAuthHeader,
} from "@common/lib/deploymentApi";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function useDeleteTables(): (
  tableNames: string[],
  componentId: string | null,
) => Promise<{ success: false; error: string } | { success: true }> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (tableNames: string[], componentId: string | null) => {
    const body = JSON.stringify({ tableNames, componentId });
    const res = await fetch(`${deploymentUrl}/api/delete_tables`, {
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
      return { success: false, error: err.message };
    }
    return { success: true };
  };
}

export const useInvalidateShapes = () => {
  const { mutate } = useSWRConfig();
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();

  return useCallback(
    async () => mutate([deploymentUrl, `/api/shapes2`, authHeader]),
    [authHeader, deploymentUrl, mutate],
  );
};

export type Index = {
  table?: string;
  name: string;
  fields:
    | string[]
    | {
        searchField: string;
        filterFields: string[];
      }
    | {
        vectorField: string;
        filterFields: string[];
        dimensions: number;
      };
  backfill: {
    state: "in_progress" | "done";
    stats?: {
      numDocsIndexed: number;
      totalDocs: number;
    };
  };
};
