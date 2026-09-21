import { HelpTooltip } from "@ui/HelpTooltip";
import { Sheet } from "@ui/Sheet";
import { cn } from "@ui/cn";

export const SHEET_ROW = "px-4 py-3";

// Table cells reach the sheet's edges, so they carry its inset themselves.
export const TABLE_CELL = "px-4 py-3 align-middle";
export const TABLE_HEADER_CELL =
  "px-4 py-2 text-left text-sm font-normal text-content-secondary";

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
  icon,
  badge,
  menu,
}: {
  title: string;
  icon?: React.ReactNode;
  /** Reads as part of the title, so it sits with it rather than at the far
      edge of the row. */
  badge?: React.ReactNode;
  menu?: React.ReactNode;
}) {
  return (
    <div className={cn(SHEET_ROW, "flex min-h-12 items-center gap-2.5")}>
      {icon}
      <h4 className="truncate">{title}</h4>
      {badge}
      <div className="ml-auto flex items-center gap-2">{menu}</div>
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
