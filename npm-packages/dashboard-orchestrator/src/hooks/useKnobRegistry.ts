import useSWR from "swr";
import { getKnobRegistry } from "../lib/orchestratorApi";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorUrl } from "../lib/config";

export function useKnobRegistry() {
  const token = useAccessToken();
  const url = orchestratorUrl();
  // Knob registry is stable per orchestrator binary — cache aggressively.
  return useSWR(
    token ? ["knobRegistry", token] : null,
    () => getKnobRegistry(url, token!),
    { revalidateOnFocus: false, revalidateIfStale: false },
  );
}
