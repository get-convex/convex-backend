import { cn } from "@ui/cn";

type CalloutVariant = "instructions" | "error" | "localDev" | "upsell";

const classes = {
  error: "bg-background-error border text-content-error",
  instructions: "border bg-background-warning text-content-warning",
  upsell: "border border-util-accent bg-util-accent/10",
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
        `mt-2 flex rounded-sm px-3 py-2 text-sm ${classes[variant]}`,
        className,
      )}
      role="alert"
      {...props}
    >
      {children}
    </div>
  );
}
