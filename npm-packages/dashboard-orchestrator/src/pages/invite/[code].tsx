import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import Link from "next/link";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { Callout } from "@ui/Callout";
import { useSession } from "../../lib/auth-client";
import { useAccessToken } from "../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../lib/config";

type State =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "success"; teamSlug: string | null }
  | { kind: "error"; message: string };

export default function InviteAcceptPage() {
  const router = useRouter();
  const code = router.query.code as string | undefined;
  const session = useSession();
  const token = useAccessToken(router.isReady ? (code ?? null) : null);
  const url = orchestratorUrl();
  const [state, setState] = useState<State>({ kind: "idle" });

  const isSignedIn = !!session?.data?.user;

  // If not signed in, send the user through the login flow with a return URL.
  // BetterAuth-issued cookies become available; on return they re-render here
  // and we proceed with accept.
  useEffect(() => {
    if (!router.isReady || !code) return;
    if (session === undefined) return; // hydrating
    if (!isSignedIn) {
      const returnTo = encodeURIComponent(`/invite/${code}`);
      void router.replace(`/login?redirect=${returnTo}`);
    }
  }, [router.isReady, code, isSignedIn, session, router]);

  const accept = async () => {
    if (!code || !token) return;
    setState({ kind: "submitting" });
    try {
      const res = await fetch(
        `${url}/api/dashboard/invites/${encodeURIComponent(code)}/accept`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${token}` },
        },
      );
      if (res.status === 404) {
        throw new Error("This invitation link is invalid or expired.");
      }
      if (res.status === 409) {
        throw new Error("This invitation has already been accepted.");
      }
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      // The accept endpoint doesn't echo the team back; refresh the user's
      // session and route them to the home page where the team picker shows
      // the new team.
      setState({ kind: "success", teamSlug: null });
      setTimeout(() => router.replace("/"), 1500);
    } catch (err) {
      setState({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  if (!router.isReady || session === undefined) {
    return <CenteredSpinner />;
  }
  if (!isSignedIn) {
    return <CenteredSpinner />;
  }
  if (!code) {
    return (
      <Centered>
        <Sheet>
          <h2 className="text-base font-semibold">Invitation not found</h2>
          <p className="mt-2 text-sm text-content-secondary">
            The invitation link is missing a code.
          </p>
        </Sheet>
      </Centered>
    );
  }

  return (
    <Centered>
      <Sheet>
        <h2 className="text-base font-semibold">Accept team invitation</h2>
        <p className="mt-2 text-sm text-content-secondary">
          You&apos;re signed in as{" "}
          <strong className="text-content-primary">
            {session.data?.user?.email}
          </strong>
          . Accepting this invitation will add you to the team.
        </p>
        {state.kind === "error" && (
          <Callout variant="error" className="mt-4">
            {state.message}
          </Callout>
        )}
        {state.kind === "success" && (
          <Callout className="mt-4">Joined! Redirecting…</Callout>
        )}
        <div className="mt-4 flex items-center gap-2">
          <Button
            onClick={accept}
            disabled={state.kind === "submitting" || state.kind === "success"}
            loading={state.kind === "submitting"}
          >
            Accept invitation
          </Button>
          <Link
            href="/"
            className="text-sm text-content-secondary underline hover:text-content-primary"
          >
            Cancel
          </Link>
        </div>
      </Sheet>
    </Centered>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <main className="flex flex-1 flex-col items-center justify-center p-6">
      <div className="w-full max-w-md">{children}</div>
    </main>
  );
}

function CenteredSpinner() {
  return (
    <Centered>
      <div className="flex justify-center py-10">
        <Spinner />
      </div>
    </Centered>
  );
}
