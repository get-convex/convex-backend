import classNames from "classnames";
import { PlanResponse } from "generatedApi";

export const planNameMap: Record<string, string> = {
  CONVEX_STARTER_PLUS: "Starter",
  CONVEX_PROFESSIONAL: "Professional",
  CONVEX_BUSINESS: "Business & Enterprise",
};

export function PlanCard({
  selected,
  plan,
  saleHeader,
  action,
}: {
  plan: PlanResponse;
  selected: boolean;
  saleHeader: React.ReactNode | string;
  action: React.ReactNode;
}) {
  return (
    <div
      className={classNames(
        "flex min-w-[12rem] flex-1 flex-col justify-between rounded-2xl border bg-background-primary/30 p-3 shadow-sm transition-colors hover:bg-background-primary/70",
        selected && "border-border-selected",
      )}
    >
      <div className="mb-2 text-content-primary">
        <h3>
          {plan.planType ? planNameMap[plan.planType] || plan.name : plan.name}
        </h3>
        <div className="text-sm text-content-secondary">{saleHeader}</div>
      </div>
      <div>{action}</div>
    </div>
  );
}
