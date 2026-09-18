import { HelpTooltip } from "@ui/HelpTooltip";
import { Sheet } from "@ui/Sheet";
import { cn } from "@ui/cn";

export const SHEET_ROW = "px-4 py-3";

export function SettingsSheet({
  title,
  description,
  descriptionTip,
  badge,
  action,
  children,
}: {
  title: string;
  description: string;
  badge?: React.ReactNode;
  descriptionTip?: React.ReactNode;
  action?: React.ReactNode;
  children?: React.ReactNode;
}) {
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-end justify-between gap-4">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <h3>{title}</h3>
            {badge}
          </div>
          <p className="max-w-prose text-sm text-content-secondary">
            {description}{" "}
            {descriptionTip && (
              <span className="inline-flex align-middle">
                <HelpTooltip>{descriptionTip}</HelpTooltip>
              </span>
            )}
          </p>
        </div>
        {action}
      </div>
      <Sheet className="flex flex-col" padding={false}>
        {children}
      </Sheet>
    </section>
  );
}

export function ConfigurationRow({
  title,
  badge,
  menu,
}: {
  title: string;
  badge?: React.ReactNode;
  menu?: React.ReactNode;
}) {
  return (
    <div className={cn(SHEET_ROW, "flex min-h-12 items-center gap-2.5")}>
      <h4 className="truncate">{title}</h4>
      <div className="ml-auto flex items-center gap-2">
        {badge}
        {menu}
      </div>
    </div>
  );
}

export function EmptyStateRow({
  message,
  action,
}: {
  message: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-6 py-4">
      <span className="max-w-prose text-content-secondary">{message}</span>
      {action}
    </div>
  );
}
