import { useEffect } from "react";
import { useRouter } from "next/router";
import useSWR from "swr";
import { ConvexLogo } from "@common/elements/ConvexLogo";
import { useOrchestratorSession } from "../lib/useOrchestratorToken";
import { listTeams } from "../lib/orchestratorApi";
import { orchestratorUrl } from "../lib/config";

/** Routes the user to /login (no session), or /t/<firstTeam> (default landing). */
export default function IndexPage() {
  const router = useRouter();
  const {
    data: session,
    error: sessionError,
    isLoading,
    isValidating,
  } = useOrchestratorSession();
  const token = session?.accessToken ?? null;

  const { data: teams, error: teamsError } = useSWR(
    token ? ["teams", token] : null,
    () => listTeams(orchestratorUrl(), token!),
  );

  useEffect(() => {
    // Never route off a session that is still settling. SWR serves the cached
    // value synchronously on mount (`keepPreviousData`), so a signed-out
    // `null` left over from before sign-in would otherwise bounce a
    // just-authenticated user back to /login while the refetch is in flight.
    if (isLoading || isValidating) return;
    // Not signed in (or BetterAuth session expired) → login.
    if (sessionError || !session) {
      void router.replace("/login");
      return;
    }
    if (teamsError) return; // SWR will retry; show "Connecting…"
    if (teams) {
      if (teams.length > 0) {
        void router.replace(`/t/${teams[0].slug}`);
      } else if (session.teamSlug) {
        void router.replace(`/t/${session.teamSlug}`);
      }
    }
  }, [
    isLoading,
    isValidating,
    sessionError,
    session,
    teams,
    teamsError,
    router,
  ]);

  return (
    <div className="flex size-full flex-col items-center justify-center gap-4">
      <ConvexLogo />
      <p className="text-sm text-content-secondary">
        {teams && teams.length === 0 ? "No teams yet." : "Connecting…"}
      </p>
    </div>
  );
}
