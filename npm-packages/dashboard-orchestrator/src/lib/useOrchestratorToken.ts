// Replaces the localStorage-based useAccessToken. Fetches the orchestrator
// PAT from the dashboard's `/api/orchestrator/token` bridge — which in turn
// requires a valid BetterAuth session cookie. The PAT lives in memory only.

import useSWR from "swr";

export type OrchestratorSession = {
  accessToken: string;
  memberId: number;
  teamSlug: string;
  role: string;
};

async function fetcher(url: string): Promise<OrchestratorSession | null> {
  const res = await fetch(url, {
    method: "GET",
    credentials: "include",
  });
  if (res.status === 401) return null;
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`orchestrator token fetch ${res.status}: ${body}`);
  }
  return (await res.json()) as OrchestratorSession;
}

export function useOrchestratorSession() {
  return useSWR<OrchestratorSession | null>(
    "/api/orchestrator/token",
    fetcher,
    {
      revalidateOnFocus: false,
      shouldRetryOnError: false,
    },
  );
}

/**
 * Convenience: returns just the access token string, or null if not yet
 * available / not authenticated.
 */
export function useAccessToken(): string | null {
  const { data } = useOrchestratorSession();
  return data?.accessToken ?? null;
}
