import useSWR from "swr";
import {
  getDeploymentSettings,
  patchDeploymentSettings,
  restartDeployment,
} from "../lib/orchestratorApi";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorUrl } from "../lib/config";

export function useDeploymentSettings(deploymentName: string | undefined) {
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { data, error, mutate } = useSWR(
    token && deploymentName
      ? ["deploymentSettings", deploymentName, token]
      : null,
    () => getDeploymentSettings(url, token!, deploymentName!),
  );
  const save = async (patch: {
    desiredTier?: string | null;
    desiredOverrides?: Record<string, string | null>;
  }) => {
    if (!token || !deploymentName) return;
    const next = await patchDeploymentSettings(
      url,
      token,
      deploymentName,
      patch,
    );
    await mutate(next, { revalidate: false });
    return next;
  };
  const restart = async (force?: boolean) => {
    if (!token || !deploymentName) return;
    await restartDeployment(url, token, deploymentName, force);
    await mutate();
  };
  return { settings: data, error, save, restart };
}
