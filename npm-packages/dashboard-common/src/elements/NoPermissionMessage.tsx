import { LockClosedIcon } from "@radix-ui/react-icons";

export function NoPermissionMessage({
  message,
  missingPermission,
}: {
  message: string;
  // Optional identifier for the permission the user is missing
  missingPermission?: string;
}) {
  return (
    <div className="flex h-full grow items-center justify-center">
      <div className="flex flex-col items-center gap-3">
        <LockClosedIcon className="size-8 text-content-tertiary" />
        <p className="text-base text-content-secondary">{message}</p>
        {missingPermission && (
          <p className="text-xs text-content-tertiary">
            Missing permission:{" "}
            <code className="rounded bg-background-tertiary px-1 py-0.5 font-mono">
              {missingPermission}
            </code>
          </p>
        )}
      </div>
    </div>
  );
}
