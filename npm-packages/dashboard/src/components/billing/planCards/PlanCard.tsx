import classNames from "classnames";
import { PlanResponse } from "generatedApi";

export const planNameMap: Record<string, string> = {
  CONVEX_STARTER_PLUS: "Starter",
  CONVEX_PROFESSIONAL: "Professional",
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
        "flex flex-col justify-between rounded-2xl border bg-background-primary/30 p-3 shadow-sm transition-colors hover:bg-background-primary/70 min-w-[18rem]",
        selected && "border-border-selected",
      )}
    >
      <div className="mb-2 text-content-primary">
        <h3>
          {plan.planType ? planNameMap[plan.planType] || plan.name : plan.name}
        </h3>
        <div className="text-base">{saleHeader}</div>
      </div>
      <div className="flex grow flex-col justify-between gap-2">
        <div className="mb-2 text-wrap text-content-secondary">
          {plan.planType === "CONVEX_BASE" && (
            <ul className="ml-4 list-disc">
              <li>For hobbyists and prototypes</li>
              <li>Up to 6 team members</li>
              <li>Up to 20 projects</li>
              <li>Projects are disabled after exceeding monthly usage limit</li>
              <li>Community-driven support on Discord</li>
            </ul>
          )}
          {plan.planType === "CONVEX_STARTER_PLUS" && (
            <ul className="ml-4 list-disc">
              <li>Everything in Free</li>
              <li>Unlocks usage-based pricing to pay as you go</li>
              <li>Community-driven support on Discord</li>
              <li>
                Perfect for small teams and Chef users that want to pay for
                resources and tokens as they go
              </li>
            </ul>
          )}
          {plan.planType === "CONVEX_PROFESSIONAL" && (
            <ul className="ml-4 list-disc">
              <li>Everything in Starter</li>
              <li>Up to 20 team members</li>
              <li>Unlimited projects</li>
              <li>Higher included usage limits</li>
              <li className="font-semibold">
                Usage-based pricing applies for usage above included limits
              </li>
              <li>Better performance</li>
              <li>Email support</li>
              <li>...and more!</li>
            </ul>
          )}
        </div>
        <div>{action}</div>
      </div>
    </div>
  );
}
