import { Snippet } from "@common/elements/Snippet";
import { Callout } from "@ui/Callout";

export function LocalDevCallout({
  children,
  tipText,
  command,
  ...props
}: {
  tipText: string;
  command?: string;
  children?: React.ReactNode;
} & React.HTMLAttributes<HTMLDivElement>) {
  const isDev = process.env.NEXT_PUBLIC_ENVIRONMENT === "development";
  if (!isDev) {
    return null;
  }
  return (
    <Callout variant="localDev" {...props}>
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
