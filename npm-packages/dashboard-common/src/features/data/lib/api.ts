import { useCallback } from "react";
import useSWR, { useSWRConfig } from "swr";
import { reportHttpError } from "@common/lib/utils";
import {
  useDeploymentUrl,
  useAdminKey,
  useDeploymentAuthHeader,
  deploymentAuthMiddleware,
  useDeploymentIsDisconnected,
} from "@common/lib/deploymentApi";
import { deploymentFetch } from "@common/lib/fetching";
import { useNents } from "@common/lib/useNents";

export function useDeleteTables(): (
  tableNames: string[],
  componentId: string | null,
) => Promise<{ success: false; error: string } | { success: true }> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

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
  table: string;
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
  };
};

export function useTableIndexes(tableName: string): {
  indexes?: Index[];
  hadError: boolean;
} {
  const { selectedNent } = useNents();
  const query = selectedNent ? `?componentId=${selectedNent.id}` : "";
  const isDisconnected = useDeploymentIsDisconnected();
  const { data, error } = useSWR<{ indexes: Index[] }>(
    isDisconnected ? null : `/api/get_indexes${query}`,
    deploymentFetch,
    {
      use: [deploymentAuthMiddleware],
      shouldRetryOnError: false,
    },
  );

  return {
    hadError: !!error,
    indexes: data?.indexes.filter((index) => index.table === tableName),
  };
}
