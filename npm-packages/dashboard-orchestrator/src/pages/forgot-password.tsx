import { useState } from "react";
import Link from "next/link";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { ConvexLogo } from "@common/elements/ConvexLogo";
import { authClient } from "../lib/auth-client";

type State =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "sent" }
  | { kind: "error"; message: string };

export default function ForgotPasswordPage() {
  const [email, setEmail] = useState("");
  const [state, setState] = useState<State>({ kind: "idle" });

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) return;
    setState({ kind: "submitting" });
    try {
      const res = await authClient.forgetPassword({
        email,
        redirectTo: "/reset-password",
      });
      if (res.error) throw new Error(res.error.message ?? "request failed");
      // Always show "sent" — never reveal whether the email was registered.
      setState({ kind: "sent" });
    } catch (err) {
      setState({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  return (
    <main className="flex h-full flex-col items-center justify-center gap-8 p-8">
      <ConvexLogo />
      <div className="w-full max-w-md">
        <Sheet>
          <h1 className="text-base font-semibold text-content-primary">
            Reset your password
          </h1>
          <p className="mt-1 text-sm text-content-secondary">
            Enter the email associated with your account. We&apos;ll send you a
            link to choose a new password.
          </p>
          {state.kind === "sent" ? (
            <Callout className="mt-4">
              If an account exists for <strong>{email}</strong>, a reset link is
              on its way. Check your inbox and the application logs (in dev, the
              link is also printed to the dashboard-orchestrator console).
            </Callout>
          ) : (
            <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3">
              <TextInput
                id="email"
                label="Email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="you@example.com"
                autoComplete="email"
              />
              {state.kind === "error" && (
                <Callout variant="error">{state.message}</Callout>
              )}
              <Button
                type="submit"
                disabled={!email || state.kind === "submitting"}
                loading={state.kind === "submitting"}
                size="xs"
                className="ml-auto w-fit"
              >
                Send reset link
              </Button>
            </form>
          )}
          <div className="mt-4 text-xs">
            <Link
              href="/login"
              className="text-content-secondary underline hover:text-content-primary"
            >
              ← Back to sign in
            </Link>
          </div>
        </Sheet>
      </div>
    </main>
  );
}
