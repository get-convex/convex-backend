import { LockClosedIcon } from "@radix-ui/react-icons";

export function NoPermissionMessage({ message }: { message: string }) {
  return (
    <div className="flex h-full grow items-center justify-center">
      <div className="flex flex-col items-center gap-3">
        <LockClosedIcon className="size-8 text-content-tertiary" />
        <p className="text-base text-content-secondary">{message}</p>
      </div>
    </div>
  );
}
