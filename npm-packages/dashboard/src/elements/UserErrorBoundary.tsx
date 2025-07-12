import { captureException } from "@sentry/nextjs";
import Link from "next/link";
import React from "react";

import { Callout } from "@ui/Callout";

export function Fallback({ eventId }: { eventId: string | null }) {
  return (
    <div className="flex w-full items-center justify-center gap-1">
      <Callout variant="error">
        <div>
          We encountered an error running your function, and the Convex
          engineering team has been notified. Try refreshing the page, or please
          contact us at{" "}
          <Link
            href="mailto:support@convex.dev"
            passHref
            className="items-center text-content-link"
          >
            support@convex.dev
          </Link>{" "}
          for support with this issue.
          <div className="mt-2">Event ID: {eventId}</div>
        </div>
      </Callout>
    </div>
  );
}

export const convexUserErrorRegex = /^Error: \[CONVEX [AMQ]\(.+\)]/;

type ErrorBoundaryWithoutReportingProps = {
  children: React.ReactNode;
  header?: React.ReactNode;
  errorKey?: string;
};

// Used in cases where you want an error boundary for exceptions caused by end-user code (like running functions in the dashboard)
// In most cases, you want to import from "@sentry/nextjs" instead
export class ErrorBoundaryWithoutReporting extends React.Component<
  ErrorBoundaryWithoutReportingProps,
  { error: any }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { error: undefined };
  }

  static getDerivedStateFromError(error: any) {
    const err = error.toString();
    if (!convexUserErrorRegex.test(err)) {
      // React doesn't send errors that occured in event handlers to
      // error boundaries, so manually send a message to Sentry and display a fallback
      // UI in the case of a real bug in the Convex client
      return { error: <Fallback eventId={captureException(err)} /> };
    }
    // Update state so the next render will show the fallback UI.
    return {
      error: (
        <div className="mt-3">
          <Callout variant="error">
            <pre>
              <code className="whitespace-pre-wrap">{err}</code>
            </pre>
          </Callout>
        </div>
      ),
    };
  }

  componentDidUpdate(prevProps: ErrorBoundaryWithoutReportingProps): void {
    const { errorKey } = this.props;
    if (errorKey !== prevProps.errorKey) {
      this.setState({ error: undefined });
    }
  }

  render() {
    const { children, header } = this.props;
    const { error } = this.state;
    if (error) {
      return (
        <>
          {header}
          {error}
        </>
      );
    }

    return children;
  }
}
