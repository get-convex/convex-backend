import { LockClosedIcon } from "@radix-ui/react-icons";
import { useHasCustomRole } from "hooks/useDeploymentPermissions";
import type { RoleStatementAction } from "@convex-dev/platform/managementApi";

export function NoPermissionMessage({
  message,
  missingPermission,
}: {
  message: string;
  missingPermission: RoleStatementAction;
}) {
  const hasCustomRole = useHasCustomRole();

  return (
    <div className="flex h-full grow items-center justify-center">
      <div className="flex flex-col items-center gap-3">
        <LockClosedIcon className="size-8 text-content-tertiary" />
        <p className="text-base text-content-secondary">{message}</p>
        {hasCustomRole && (
          <p className="text-xs text-content-tertiary">
            Missing permission:{" "}
            <code className="rounded-sm bg-background-tertiary px-1 py-0.5 font-mono">
              {missingPermission}
            </code>
          </p>
        )}
      </div>
    </div>
  );
}
