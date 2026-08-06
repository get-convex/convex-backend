// Sign-in / sign-up page styled after Convex Cloud's auth.convex.dev:
// orb logo, credential form, then a horizontal OR divider with social-login
// buttons (Google + GitHub) and a "Last used" pill on whichever the user last
// picked. Mirrors the reference shot the user supplied so the orchestrator's
// auth screen feels indistinguishable from cloud's.

import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import {
  EnterIcon,
  EyeNoneIcon,
  EyeOpenIcon,
  GitHubLogoIcon,
} from "@radix-ui/react-icons";
import Link from "next/link";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { useSWRConfig } from "swr";
import { authClient } from "../lib/auth-client";
import {
  ORCHESTRATOR_SESSION_KEY,
  fetchOrchestratorSession,
} from "../lib/useOrchestratorToken";
import { ConvexOrb } from "../components/ConvexOrb";

type Mode = "signin" | "signup";

const LAST_PROVIDER_KEY = "orch-last-auth-provider";

// Whitelist redirect targets so an attacker can't smuggle in absolute URLs
// (`?redirect=https://evil.com`) and bounce signed-in users elsewhere.
function safeRedirect(raw: string | string[] | undefined): string {
  if (typeof raw !== "string") return "/";
  if (!raw.startsWith("/") || raw.startsWith("//")) return "/";
  return raw;
}

export default function LoginPage() {
  const router = useRouter();
  const { mutate } = useSWRConfig();
  const redirectTo = safeRedirect(router.query.redirect);
  const [mode, setMode] = useState<Mode>("signin");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [name, setName] = useState("");
  const [showSecret, setShowSecret] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const githubEnabled = process.env.NEXT_PUBLIC_ENABLE_GITHUB_LOGIN === "true";
  const googleEnabled = process.env.NEXT_PUBLIC_ENABLE_GOOGLE_LOGIN === "true";

  const [lastProvider, setLastProvider] = useState<string | null>(null);
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      setLastProvider(window.localStorage.getItem(LAST_PROVIDER_KEY));
    } catch {
      // Firefox throws "operation is insecure" under strict tracking
      // protection. Treat as no remembered provider.
    }
  }, []);

  const remember = (provider: string) => {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(LAST_PROVIDER_KEY, provider);
    } catch {
      // See useEffect above.
    }
  };

  const onSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const submittedEmail = String(formData.get("email") ?? "").trim();
    const submittedPassword = String(formData.get("password") ?? "");
    const submittedName = String(formData.get("name") ?? "").trim();

    setError(null);
    setEmail(submittedEmail);
    setPassword(submittedPassword);
    setName(submittedName);

    if (!submittedEmail || !submittedPassword) {
      setError("Email and password are required.");
      return;
    }
    if (mode === "signup" && submittedPassword.length < 8) {
      setError("Password must be at least 8 characters.");
      return;
    }

    setSubmitting(true);
    try {
      if (mode === "signin") {
        const res = await authClient.signIn.email({
          email: submittedEmail,
          password: submittedPassword,
        });
        if (res.error) throw new Error(res.error.message ?? "sign-in failed");
      } else {
        const res = await authClient.signUp.email({
          email: submittedEmail,
          password: submittedPassword,
          name: submittedName || submittedEmail.split("@")[0],
        });
        if (res.error) throw new Error(res.error.message ?? "sign-up failed");
      }
      remember("email");
      // While signed out, the session key is cached as `null`, and `_app.tsx`
      // installs a 30s `dedupingInterval`. Navigating straight to `/` would
      // let IndexPage read that stale `null` — no refetch happens inside the
      // dedupe window — conclude the user is signed out, and bounce right
      // back here. (That's why signing in only "worked" after a manual
      // refresh, which cleared the in-memory cache.) Write the new session
      // into the cache and wait for it before routing. `revalidate: false`
      // because we're supplying the authoritative value ourselves.
      await mutate(ORCHESTRATOR_SESSION_KEY, fetchOrchestratorSession(), {
        revalidate: false,
      });
      void router.replace(redirectTo);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  const onSocial = async (provider: "github" | "google") => {
    setError(null);
    remember(provider);
    try {
      await authClient.signIn.social({ provider, callbackURL: redirectTo });
    } catch (err) {
      setError((err as Error).message);
    }
  };

  const submitLabel = mode === "signin" ? "Sign in" : "Create account";

  return (
    <main className="flex min-h-screen w-full flex-col items-center justify-center gap-6 bg-background-primary p-6">
      <ConvexOrb size={56} />
      {/* eslint-disable-next-line no-restricted-syntax -- text-2xl IS the heading style on an h1 */}
      <h1 className="text-2xl font-semibold text-content-primary">
        {mode === "signin" ? "Sign in to Convex" : "Create your account"}
      </h1>
      <div className="w-full max-w-md rounded-lg border border-border-transparent bg-background-secondary p-6 shadow-sm">
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <TextInput
            id="email"
            label="Email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="Your email address"
            autoComplete={mode === "signin" ? "username" : "email"}
            autoFocus
          />
          {mode === "signup" && (
            <TextInput
              id="name"
              label="Name (optional)"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Your name"
              autoComplete="name"
            />
          )}
          <TextInput
            id="password"
            label="Password"
            type={showSecret ? "text" : "password"}
            Icon={showSecret ? EyeNoneIcon : EyeOpenIcon}
            value={password}
            action={() => setShowSecret(!showSecret)}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete={
              mode === "signin" ? "current-password" : "new-password"
            }
            description={
              mode === "signup" ? "Minimum 8 characters." : undefined
            }
          />
          {error && (
            <div className="text-xs text-content-error" role="alert">
              {error}
            </div>
          )}
          <Button
            type="submit"
            variant="neutral"
            icon={<EnterIcon />}
            disabled={submitting}
            className="w-full"
          >
            {submitLabel}
          </Button>
          {mode === "signin" && (
            <Link
              href="/forgot-password"
              className="self-end text-xs text-content-secondary underline hover:text-content-primary"
            >
              Forgot password?
            </Link>
          )}
        </form>

        {(githubEnabled || googleEnabled) && (
          <>
            <div className="my-4 flex items-center gap-2 text-xs text-content-secondary">
              <hr className="flex-1 border-border-transparent" />
              <span>OR</span>
              <hr className="flex-1 border-border-transparent" />
            </div>
            <div className="flex flex-col gap-2">
              {googleEnabled && (
                <SocialButton
                  provider="google"
                  lastUsed={lastProvider === "google"}
                  onClick={() => void onSocial("google")}
                />
              )}
              {githubEnabled && (
                <SocialButton
                  provider="github"
                  lastUsed={lastProvider === "github"}
                  onClick={() => void onSocial("github")}
                />
              )}
            </div>
          </>
        )}
      </div>
      <p className="text-sm text-content-secondary">
        {mode === "signin" ? (
          <>
            Don&apos;t have an account?{" "}
            {/* eslint-disable-next-line react/forbid-elements -- inline mode toggle styled as a hyperlink, intentional plain <button> */}
            <button
              type="button"
              onClick={() => {
                setMode("signup");
                setError(null);
                setPassword("");
              }}
              // eslint-disable-next-line no-restricted-syntax -- styled as a link inline; @ui/Link doesn't support button semantics
              className="text-content-link underline"
            >
              Sign up
            </button>
          </>
        ) : (
          <>
            Already have an account?{" "}
            {/* eslint-disable-next-line react/forbid-elements -- inline mode toggle styled as a hyperlink, intentional plain <button> */}
            <button
              type="button"
              onClick={() => {
                setMode("signin");
                setError(null);
                setPassword("");
              }}
              // eslint-disable-next-line no-restricted-syntax -- styled as a link inline; @ui/Link doesn't support button semantics
              className="text-content-link underline"
            >
              Sign in
            </button>
          </>
        )}
      </p>
    </main>
  );
}

function SocialButton({
  provider,
  lastUsed,
  onClick,
}: {
  provider: "github" | "google";
  lastUsed: boolean;
  onClick: () => void;
}) {
  const label =
    provider === "google" ? "Continue with Google" : "Continue with GitHub";
  return (
    <div className="relative">
      {lastUsed && (
        <span className="absolute -top-2 right-3 z-10 rounded-md bg-background-tertiary px-1.5 py-0.5 text-[10px] text-content-secondary">
          Last used
        </span>
      )}
      <Button
        type="button"
        variant="neutral"
        onClick={onClick}
        className="w-full"
      >
        <span className="flex items-center justify-center gap-2">
          {provider === "google" ? <GoogleGlyph /> : <GitHubLogoIcon />}
          {label}
        </span>
      </Button>
    </div>
  );
}

function GoogleGlyph() {
  return (
    <svg width="16" height="16" viewBox="0 0 48 48" aria-hidden>
      <path
        fill="#FFC107"
        d="M43.6 20.5h-1.9V20H24v8h11.3c-1.6 4.5-5.9 7.8-11.3 7.8-6.6 0-12-5.4-12-12s5.4-12 12-12c3 0 5.7 1.1 7.8 2.9l5.7-5.7C34 6.1 29.3 4 24 4 12.9 4 4 12.9 4 24s8.9 20 20 20 20-8.9 20-20c0-1.2-.1-2.4-.4-3.5z"
      />
      <path
        fill="#FF3D00"
        d="M6.3 14.7l6.6 4.8C14.7 15.1 19 12 24 12c3 0 5.7 1.1 7.8 2.9l5.7-5.7C34 6.1 29.3 4 24 4 16.5 4 10 8.3 6.3 14.7z"
      />
      <path
        fill="#4CAF50"
        d="M24 44c5.2 0 9.9-2 13.5-5.2l-6.2-5.2c-2 1.4-4.6 2.4-7.3 2.4-5.4 0-10-3.5-11.6-8.3L6 32.7C9.6 39.3 16.3 44 24 44z"
      />
      <path
        fill="#1976D2"
        d="M43.6 20.5H24v8h11.3c-.8 2.2-2.2 4-4.1 5.3l6.2 5.2C40.7 36.6 44 31.6 44 24c0-1.2-.1-2.4-.4-3.5z"
      />
    </svg>
  );
}
