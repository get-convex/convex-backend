import { cn } from "@ui/cn";

export function ConvexSchemaFilePath({ className }: { className?: string }) {
  return (
    <code
      className={cn(
        "rounded-sm bg-background-tertiary p-1 text-sm text-content-primary",
        className,
      )}
    >
      convex/schema.ts
    </code>
  );
}
