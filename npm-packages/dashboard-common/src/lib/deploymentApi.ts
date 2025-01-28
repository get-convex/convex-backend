import { useCallback, useContext, useMemo } from "react";
import { Shape, shapeSchema } from "shapes";
import useSWR, { BareFetcher, Middleware, useSWRConfig } from "swr";
import { z } from "zod";
import { useNents } from "./useNents";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "./deploymentContext";
import { deploymentFetch } from "./fetching";
import { isUserTableName } from "./utils";
import { displayName } from "./functions/generateFileTree";

export function useDeploymentUrl(): string {
  const { deployment } = useContext(ConnectedDeploymentContext);
  if (!deployment) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  return deployment.deploymentUrl;
}

export function useDeploymentAuthHeader() {
  const { deployment } = useContext(ConnectedDeploymentContext);
  if (!deployment) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  return `Convex ${deployment.adminKey}`;
}

export function useAdminKey() {
  const { deployment } = useContext(ConnectedDeploymentContext);
  if (!deployment) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  return deployment.adminKey;
}

export function useDeploymentIsDisconnected(): boolean {
  const value = useContext(ConnectedDeploymentContext);
  if (!value) {
    throw Error("Must be used inside a loaded connected deployment!");
  }
  return value.isDisconnected;
}

const shapes2ResponseSchema = z.record(shapeSchema);

export function useTableShapes(): {
  tables: Map<string, Shape> | undefined;
  hadError: boolean;
} {
  const { selectedNent } = useNents();
  const componentQuery = selectedNent ? `?component=${selectedNent.id}` : "";
  const isDisconnected = useDeploymentIsDisconnected();
  const fetcher: BareFetcher = deploymentFetch;
  const { data, error } = useSWR(
    isDisconnected ? null : `/api/shapes2${componentQuery}`,
    fetcher,
    {
      use: [deploymentAuthMiddleware],
      refreshWhenHidden: true,
      refreshInterval: 5000,
    },
  );
  return {
    tables: useMemo(() => {
      if (!data) return undefined;

      const shapes = shapes2ResponseSchema.parse(data);
      return new Map(
        Object.entries(shapes)
          .sort()
          .filter(([name]) => isUserTableName(name)),
      );
    }, [data]),
    hadError: !!error,
  };
}

export function useInvalidateSourceCode() {
  const { mutate } = useSWRConfig();
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();

  return useCallback(
    async (componentId: string | null, path: string) => {
      const componentQuery = componentId ? `&component=${componentId}` : "";
      return mutate([
        deploymentUrl,
        `/api/get_source_code?path=${path}${componentQuery}`,
        authHeader,
      ]);
    },
    [authHeader, deploymentUrl, mutate],
  );
}

// componentId: undefined means use the current selected component.
// componentId: null means use the root component.
export function useFunctionUrl(udfPath: string, componentId?: string | null) {
  const { deploymentsURI } = useContext(DeploymentInfoContext);

  const display = displayName(udfPath);
  const { selectedNent } = useNents();
  const nent = componentId !== undefined ? componentId : selectedNent?.id;
  const componentQuery = nent ? `&component=${nent}` : "";
  return `${deploymentsURI}functions?function=${display}${componentQuery}`;
}

export const deploymentAuthMiddleware: Middleware =
  (useSWRNext) => (key, fetcher, config) => {
    let swrKey = key;
    const deploymentUrl = useDeploymentUrl();
    const adminKey = useAdminKey();
    const authHeader = `Convex ${adminKey}`;
    if (!key) {
      swrKey = null;
    } else {
      swrKey = [deploymentUrl, key, authHeader];
    }

    return useSWRNext(swrKey, fetcher, config);
  };
