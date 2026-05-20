import useSWR from "swr";
import { getHostCapacity } from "../lib/orchestratorApi";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorUrl } from "../lib/config";

export function useHostCapacity() {
  const token = useAccessToken();
  const url = orchestratorUrl();
  return useSWR(token ? ["hostCapacity", token] : null, () =>
    getHostCapacity(url, token!),
  );
}
