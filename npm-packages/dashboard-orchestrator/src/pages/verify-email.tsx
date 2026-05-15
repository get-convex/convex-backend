import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import Link from "next/link";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { Spinner } from "@ui/Spinner";
import { ConvexLogo } from "@common/elements/ConvexLogo";

type State =
  | { kind: "verifying" }
  | { kind: "success" }
  | { kind: "error"; message: string };

export default function VerifyEmailPage() {
  const router = useRouter();
  const [state, setState] = useState<State>({ kind: "verifying" });

  useEffect(() => {
    if (!router.isReady) return;
    const token = router.query.token;
    if (typeof token !== "string") {
      setState({ kind: "error", message: "Missing verification token." });
      return;
    }
    // BetterAuth's verify-email endpoint is a GET on the API route, with the
    // token in the query string. Hitting it server-side validates and
    // optionally signs the user in (per autoSignInAfterVerification).
    void (async () => {
      try {
        const res = await fetch(
          `/api/auth/verify-email?token=${encodeURIComponent(token)}`,
          { credentials: "include" },
        );
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        setState({ kind: "success" });
        setTimeout(() => router.replace("/"), 1500);
      } catch (err) {
        setState({
          kind: "error",
          message: err instanceof Error ? err.message : String(err),
        });
      }
    })();
  }, [router.isReady, router, router.query.token]);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-8 p-8">
      <ConvexLogo />
      <div className="w-full max-w-md">
        <Sheet>
          <h1 className="text-base font-semibold text-content-primary">
            {state.kind === "verifying"
              ? "Verifying your email…"
              : state.kind === "success"
                ? "Email verified"
                : "Verification failed"}
          </h1>
          {state.kind === "verifying" && (
            <div className="mt-4 flex justify-center">
              <Spinner />
            </div>
          )}
          {state.kind === "success" && (
            <Callout className="mt-4">
              Thanks! You&apos;re all set. Redirecting…
            </Callout>
          )}
          {state.kind === "error" && (
            <>
              <Callout variant="error" className="mt-4">
                {state.message}
              </Callout>
              <Link
                href="/login"
                className="mt-4 inline-block text-xs text-content-secondary underline hover:text-content-primary"
              >
                ← Back to sign in
              </Link>
            </>
          )}
        </Sheet>
      </div>
    </main>
  );
}
