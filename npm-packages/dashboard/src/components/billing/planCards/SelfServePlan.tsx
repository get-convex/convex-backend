import { PlanResponse } from "generatedApi";
import { PlanCard } from "./PlanCard";

export function SelfServePlan({
  currentPlan,
  percentOff,
  plan,
  action,
}: {
  currentPlan?: string;
  plan: PlanResponse;
  percentOff?: number;
  action?: React.ReactNode;
}) {
  return (
    <PlanCard
      selected={currentPlan === plan.id}
      plan={plan}
      saleHeader={
        plan.seatPrice ? (
          percentOff ? (
            <>
              <span className="mr-1 line-through">${plan.seatPrice}</span>$
              {Number((plan.seatPrice * (1 - percentOff / 100)).toFixed(2))}
            </>
          ) : (
            `$${plan.seatPrice}`
          )
        ) : (
          "Pay as you go"
        )
      }
      saleSubheader={plan.seatPrice ? "per member, per month" : "Pay as you go"}
      action={action}
    />
  );
}
