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

/**
 * SWR cache key for the plain (no invite code) session lookup. Exported so
 * sign-in can invalidate the cached signed-out session by key rather than
 * re-spelling the URL — see `pages/login.tsx`.
 */
export const ORCHESTRATOR_SESSION_KEY = "/api/orchestrator/token";

/**
 * One-shot session fetch, outside of React. Sign-in feeds the result straight
 * into the SWR cache so the next page reads the new session rather than the
 * stale signed-out one — a bare `mutate(key)` only marks the entry for
 * revalidation, which does nothing while no component is subscribed to it.
 */
export function fetchOrchestratorSession(): Promise<OrchestratorSession | null> {
  return fetcher(ORCHESTRATOR_SESSION_KEY);
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
        ? `${ORCHESTRATOR_SESSION_KEY}?inviteCode=${encodeURIComponent(inviteCode)}`
        : ORCHESTRATOR_SESSION_KEY;

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
