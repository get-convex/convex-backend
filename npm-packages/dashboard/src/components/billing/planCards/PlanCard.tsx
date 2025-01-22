import { PlanResponse } from "generatedApi";

export function PlanCard({
  selected,
  plan,
  saleHeader,
  saleSubheader,
  action,
}: {
  plan: PlanResponse;
  selected: boolean;
  saleHeader: React.ReactNode | string;
  saleSubheader?: string;
  action: React.ReactNode;
}) {
  return (
    <div
      className={`h-full rounded border p-3 ${
        selected && "bg-background-tertiary"
      }`}
    >
      <div className="mb-4 text-content-primary">
        <div className="text-xs">{plan.name}</div>
        <div className="flex flex-wrap items-end gap-1">
          <div className="text-base">{saleHeader}</div>
          <div className="text-xs leading-6 text-content-secondary">
            {saleSubheader}
          </div>
        </div>
      </div>
      <div>
        <div className="mb-2 h-[3.75rem] text-content-secondary">
          {plan.description}
        </div>
        {action}
      </div>
    </div>
  );
}
