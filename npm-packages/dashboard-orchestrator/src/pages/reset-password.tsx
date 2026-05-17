import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import Link from "next/link";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { EyeNoneIcon, EyeOpenIcon } from "@radix-ui/react-icons";
import { ConvexLogo } from "@common/elements/ConvexLogo";
import { authClient } from "../lib/auth-client";

type State =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "success" }
  | { kind: "error"; message: string };

export default function ResetPasswordPage() {
  const router = useRouter();
  const [token, setToken] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [showSecret, setShowSecret] = useState(false);
  const [state, setState] = useState<State>({ kind: "idle" });

  // BetterAuth places the token in `?token=` after redirect.
  useEffect(() => {
    if (!router.isReady) return;
    const t = router.query.token;
    setToken(typeof t === "string" ? t : null);
  }, [router.isReady, router.query.token]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token || password.length < 8) return;
    setState({ kind: "submitting" });
    try {
      const res = await authClient.resetPassword({
        newPassword: password,
        token,
      });
      if (res.error) throw new Error(res.error.message ?? "reset failed");
      setState({ kind: "success" });
      setTimeout(() => router.replace("/login"), 1500);
    } catch (err) {
      setState({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  if (!router.isReady) return null;

  if (token === null) {
    return (
      <main className="flex h-full flex-col items-center justify-center gap-8 p-8">
        <ConvexLogo />
        <div className="w-full max-w-md">
          <Sheet>
            <h1 className="text-base font-semibold text-content-primary">
              Invalid reset link
            </h1>
            <p className="mt-2 text-sm text-content-secondary">
              This password-reset link is missing its token. Request a new one.
            </p>
            <Link
              href="/forgot-password"
              // eslint-disable-next-line no-restricted-syntax -- using Next.js Link, the rule's target is plain <a class="text-content-link"> only
              className="mt-4 inline-block text-sm text-content-link underline"
            >
              Request a new link
            </Link>
          </Sheet>
        </div>
      </main>
    );
  }

  return (
    <main className="flex h-full flex-col items-center justify-center gap-8 p-8">
      <ConvexLogo />
      <div className="w-full max-w-md">
        <Sheet>
          <h1 className="text-base font-semibold text-content-primary">
            Choose a new password
          </h1>
          {state.kind === "success" ? (
            <Callout className="mt-4">
              Password updated. Redirecting to sign in…
            </Callout>
          ) : (
            <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3">
              <TextInput
                id="newPassword"
                label="New password"
                type={showSecret ? "text" : "password"}
                Icon={showSecret ? EyeNoneIcon : EyeOpenIcon}
                value={password}
                action={() => setShowSecret(!showSecret)}
                onChange={(e) => setPassword(e.target.value)}
                description="Minimum 8 characters."
                autoComplete="new-password"
              />
              {state.kind === "error" && (
                <Callout variant="error">{state.message}</Callout>
              )}
              <Button
                type="submit"
                disabled={password.length < 8 || state.kind === "submitting"}
                loading={state.kind === "submitting"}
                size="xs"
                className="ml-auto w-fit"
              >
                Set new password
              </Button>
            </form>
          )}
        </Sheet>
      </div>
    </main>
  );
}
