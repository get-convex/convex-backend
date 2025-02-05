import { cn } from "@common/lib/cn";

export function ConvexSchemaFilePath({ className }: { className?: string }) {
  return (
    <code
      className={cn(
        "rounded bg-background-tertiary p-1 text-sm text-content-primary",
        className,
      )}
    >
      convex/schema.ts
    </code>
  );
}
