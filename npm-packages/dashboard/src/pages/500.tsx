import { captureMessage } from "@sentry/nextjs";
import { Callout } from "@ui/Callout";
import { Link } from "@ui/Link";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { LockClosedIcon } from "@radix-ui/react-icons";
import { useMyCustomRoles } from "api/roles";
import { useCurrentTeam } from "api/teams";
import { useSupportFormOpen } from "elements/SupportWidget";
import { DEPLOYMENT_OP_TO_ACTION } from "lib/permissions";

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
  // Surface custom-role denials with a tailored message. The role list
  // is mirrored to the deployment's admin key out-of-band, so a member
  // whose role was just updated may briefly hit this until the new
  // grants propagate. The server formats the error as
  // "You do not have permission to perform this operation ({action})" —
  // pull the action out so the UI can show which permission was missing.
  const permissionDeniedMatch = error.message.match(
    /You do not have permission to perform this operation(?:\s*\(([^)]+)\))?/,
  );
  if (permissionDeniedMatch) {
    const rawOperation = permissionDeniedMatch[1]?.trim();
    // Translate deployment-op names to their custom-role equivalents
    // (e.g. "ViewData" → "deployment:data:view"); fall through to the
    // raw value so action strings already in the role format pass
    // through unchanged.
    const missingAction = rawOperation
      ? (DEPLOYMENT_OP_TO_ACTION[
          rawOperation as keyof typeof DEPLOYMENT_OP_TO_ACTION
        ] ?? rawOperation)
      : undefined;
    return <PermissionDeniedFallback missingAction={missingAction} />;
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
                className="items-center"
              >
                support@convex.dev
              </Link>{" "}
              for support with this issue.
            </p>
            {eventId !== null && <div>Event ID: {eventId}</div>}{" "}
            <Link href="https://status.convex.dev">Convex Status page</Link>
          </div>
        </Callout>
      </div>
    </div>
  );
}

// Split out so the role-lookup hooks (`useMyCustomRoles` chains
// `useTeamMembers` + profile fetches) only fire on the permission-denied
// branch — the generic 500 page may render outside an authenticated
// context where those queries would be useless.
function PermissionDeniedFallback({
  missingAction,
}: {
  missingAction?: string;
}) {
  const team = useCurrentTeam();
  const myRoles = useMyCustomRoles(team?.id);
  const [, setSupportFormOpen] = useSupportFormOpen();
  const isCustomRole = myRoles?.role === "custom";

  return (
    <div className="h-full grow">
      <div className="flex h-full flex-col items-center justify-center p-6">
        <Sheet className="flex max-w-prose flex-col items-center gap-3 text-center">
          <LockClosedIcon className="size-8 text-content-tertiary" />
          <p className="text-base text-content-secondary">
            You do not have permission to perform this operation.
          </p>
          {missingAction && (
            <p className="text-xs text-content-tertiary">
              Missing permission:{" "}
              <code className="rounded bg-background-tertiary px-1 py-0.5 font-mono">
                {missingAction}
              </code>
            </p>
          )}
          <p className="text-sm text-content-secondary">
            If your role was just updated, it may take a few minutes for the
            changes to propagate to this deployment. Try again shortly.
          </p>
          {isCustomRole && (
            <p className="text-sm text-content-secondary">
              Custom Roles are currently in beta.{" "}
              <Button
                inline
                variant="unstyled"
                className="underline"
                onClick={() =>
                  setSupportFormOpen({
                    defaultSubject: "Custom roles issue",
                    defaultMessage: missingAction
                      ? `I hit a permission denial on a deployment using a custom role.\n\nMissing permission: ${missingAction}\n\n[Describe what you were trying to do]`
                      : "I hit a permission denial on a deployment using a custom role.\n\n[Describe what you were trying to do]",
                  })
                }
              >
                Report an issue
              </Button>
              .
            </p>
          )}
        </Sheet>
      </div>
    </div>
  );
}
