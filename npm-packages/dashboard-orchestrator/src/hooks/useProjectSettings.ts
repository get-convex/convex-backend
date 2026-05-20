import useSWR from "swr";
import {
  getProjectSettings,
  patchProjectSettings,
} from "../lib/orchestratorApi";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorUrl } from "../lib/config";

export function useProjectSettings(projectId: number | undefined) {
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { data, error, mutate } = useSWR(
    token && projectId ? ["projectSettings", projectId, token] : null,
    () => getProjectSettings(url, token!, projectId!),
  );
  const save = async (patch: {
    tier?: string;
    knobOverrides?: Record<string, string | null>;
  }) => {
    if (!token || !projectId) return;
    const next = await patchProjectSettings(url, token, projectId, patch);
    await mutate(next, { revalidate: false });
    return next;
  };
  return { settings: data, error, save };
}
