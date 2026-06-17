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
  return useOrchestratorSessionForInvite(undefined);
}

export function useOrchestratorSessionForInvite(
  inviteCode: string | null | undefined,
) {
  const key =
    inviteCode === null
      ? null
      : inviteCode
        ? `/api/orchestrator/token?inviteCode=${encodeURIComponent(inviteCode)}`
        : "/api/orchestrator/token";

  return useSWR<OrchestratorSession | null>(key, fetcher, {
    revalidateOnFocus: false,
    shouldRetryOnError: false,
  });
}

/**
 * Convenience: returns just the access token string, or null if not yet
 * available / not authenticated.
 */
export function useAccessToken(inviteCode?: string | null): string | null {
  const { data } = useOrchestratorSessionForInvite(inviteCode);
  return data?.accessToken ?? null;
}
