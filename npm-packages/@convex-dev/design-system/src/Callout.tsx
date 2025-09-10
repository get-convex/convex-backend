import { cn } from "@ui/cn";

type CalloutVariant = "instructions" | "error" | "hint" | "localDev" | "upsell";

const classes: Record<CalloutVariant, string> = {
  error: "bg-background-error text-content-error",
  instructions: "bg-background-warning text-content-warning",
  hint: "bg-util-accent/10",
  upsell: "bg-util-accent/10 dark:bg-util-accent/30",
  localDev:
    "bg-teal-100 border border-teal-500 dark:bg-teal-900 text-content-primary",
};

export function Callout({
  variant = "instructions",
  children,
  className,
  ...props
}: {
  variant?: CalloutVariant;
  children: React.ReactNode;
} & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        `mt-2 flex rounded-lg p-3 text-sm ${classes[variant]}`,
        className,
      )}
      role="alert"
      {...props}
    >
      {children}
    </div>
  );
}
