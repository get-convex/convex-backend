import { cn } from "../lib/cn";
import { Snippet } from "./Snippet";

type CalloutVariant = "instructions" | "error" | "localDev" | "upsell";

const classes = {
  error: "bg-background-error border text-content-error",
  instructions: "border bg-background-warning text-content-warning",
  upsell: "border border-util-accent max-w-prose bg-util-accent/10",
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
        `flex mt-2 px-3 py-2 rounded text-sm ${classes[variant]}`,
        className,
      )}
      role="alert"
      {...props}
    >
      {children}
    </div>
  );
}

export function LocalDevCallout({
  variant = "localDev",
  children,
  tipText,
  command,
  ...props
}: {
  variant?: CalloutVariant;
  tipText: string;
  command?: string;
  children?: React.ReactNode;
} & React.HTMLAttributes<HTMLDivElement>) {
  const isDev = process.env.NEXT_PUBLIC_ENVIRONMENT === "development";
  if (!isDev) {
    return null;
  }
  return (
    <Callout variant={variant} {...props}>
      <div className="grow flex-col text-xs">
        {tipText}
        {command && (
          <Snippet
            value={command}
            monospace
            copying="Command"
            className="grow"
          />
        )}
      </div>
      {children}
    </Callout>
  );
}
