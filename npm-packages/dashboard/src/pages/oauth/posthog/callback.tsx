import { useEffect, useState } from "react";

const EXPECTED_MESSAGE_TYPE = "convex-posthog-oauth-callback";

export default function PostHogOAuthCallback() {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!window.opener) {
      setError("This window must be opened from the Convex dashboard.");
      return;
    }

    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    const state = params.get("state");
    const oauthError = params.get("error");
    const oauthErrorDescription = params.get("error_description");

    window.opener.postMessage(
      {
        type: EXPECTED_MESSAGE_TYPE,
        code,
        state,
        error: oauthError,
        errorDescription: oauthErrorDescription,
      },
      window.location.origin,
    );
    window.close();
  }, []);

  return (
    <div className="flex h-screen items-center justify-center p-4 text-center text-sm">
      {error ?? "Completing PostHog authorization…"}
    </div>
  );
}
