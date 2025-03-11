import { captureMessage } from "@sentry/nextjs";
import { Callout } from "dashboard-common/elements/Callout";
import Link from "next/link";

export default function Custom500() {
  return <Fallback eventId={null} />;
}

export function Fallback({ eventId }: { eventId: string | null }) {
  captureMessage("ErrorBoundary triggered");
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
