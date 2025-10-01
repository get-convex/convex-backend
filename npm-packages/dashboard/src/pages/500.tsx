import { captureMessage } from "@sentry/nextjs";
import { Callout } from "@ui/Callout";
import Link from "next/link";

export default function Custom500() {
  return <Fallback eventId={null} error={new Error("Internal Server Error")} />;
}

export function Fallback({
  eventId,
  error,
}: {
  eventId: string | null;
  error: Error;
}) {
  captureMessage("ErrorBoundary triggered", "info");
  if (
    error.message.includes("Couldn't find system module") ||
    /Couldn't find ".+" in module/.test(error.message)
  ) {
    return (
      <div className="h-full grow">
        <div className="flex h-full flex-col items-center justify-center">
          <Callout variant="error" className="max-w-prose">
            <span>
              Your local deployment is out of date. Please restart it with{" "}
              <code>npx convex dev</code> and upgrade.
            </span>
          </Callout>
        </div>
      </div>
    );
  }
  return (
    <div className="h-full grow">
      <div className="flex h-full flex-col items-center justify-center">
        <Callout variant="error">
          <div className="flex flex-col gap-2">
            <p>We encountered an error loading this page.</p>
            <p>
              {" "}
              Please try again or contact us at{" "}
              <Link
                href="mailto:support@convex.dev"
                passHref
                className="items-center text-content-link"
              >
                support@convex.dev
              </Link>{" "}
              for support with this issue.
            </p>
            {eventId !== null && <div>Event ID: {eventId}</div>}{" "}
            <Link
              href="https://status.convex.dev"
              className="text-content-link hover:underline"
            >
              Convex Status page
            </Link>
          </div>
        </Callout>
      </div>
    </div>
  );
}
