import { useEffect, useState } from "react";

const MESSAGE_TYPE = "convex-posthog-oauth-callback";

export function PostHogOAuthCallbackPage() {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!window.opener) {
      setError("This window must be opened from the Convex dashboard.");
      return;
    }

    const params = new URLSearchParams(window.location.search);
    window.opener.postMessage(
      {
        type: MESSAGE_TYPE,
        code: params.get("code"),
        state: params.get("state"),
        error: params.get("error"),
        errorDescription: params.get("error_description"),
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
