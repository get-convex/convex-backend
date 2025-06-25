import { planNameMap } from "components/billing/planCards/PlanCard";
import { PlanResponse } from "generatedApi";
import Link from "next/link";
import startCase from "lodash/startCase";
import { Callout } from "@ui/Callout";

export function PriceSummary({
  plan,
  teamMemberDiscountPct,
  numMembers,
  couponDurationInMonths,
  requiresPaymentMethod,
  isUpgrading,
  teamManagedBy,
}: {
  plan: PlanResponse;
  teamMemberDiscountPct: number;
  numMembers: number;
  couponDurationInMonths?: number;
  requiresPaymentMethod: boolean;
  isUpgrading: boolean;
  teamManagedBy?: string;
}) {
  const newPlanName = plan.planType
    ? planNameMap[plan.planType] || plan.name
    : plan.name;
  return (
    <div className="flex flex-col gap-2 text-sm" data-testid="price-summary">
      {teamManagedBy && (
        <Callout className="mb-2">
          This team's billing is currently being managed by{" "}
          {startCase(teamManagedBy)}. Upgrading to this plan will disable your{" "}
          {startCase(teamManagedBy)} integration and migrate billing to be
          handled by Convex.
        </Callout>
      )}
      {plan.seatPrice ? (
        <>
          <p>
            The {newPlanName} plan costs{" "}
            <PriceInDollars
              price={plan.seatPrice}
              percentOff={!requiresPaymentMethod ? 1 : teamMemberDiscountPct}
            />{" "}
            per team member, per month
          </p>
          <p>
            Usage-based charges will apply for all usage exceeding the limits
            included with this plan.
          </p>
        </>
      ) : (
        <p className="max-w-prose">
          {newPlanName} is a "pay as you go" plan. You'll be charged for usage
          above the included limits of this plan. See the{" "}
          <Link
            href="https://convex.dev/pricing"
            target="_blank"
            className="text-content-link hover:underline"
          >
            pricing page
          </Link>{" "}
          for more details on usage-based pricing.
        </p>
      )}
      {couponDurationInMonths !== undefined && couponDurationInMonths > 0 && (
        <p>
          This discount will be applied for the next {couponDurationInMonths}{" "}
          months.
        </p>
      )}
      {requiresPaymentMethod && plan.seatPrice && (
        <p>
          Your team has {numMembers} member{numMembers > 1 && "s"}. Once you
          upgrade, you'll be immediately charged{" "}
          {isUpgrading ? (
            "a prorated amount for each team member for the remaining time in the current billing cycle"
          ) : (
            <PriceInDollars
              price={plan.seatPrice! * numMembers}
              percentOff={teamMemberDiscountPct}
            />
          )}
          .
        </p>
      )}
    </div>
  );
}

export function PriceInDollars({
  price,
  percentOff,
}: {
  price: number;
  percentOff: number;
}) {
  return percentOff ? (
    <>
      <span className="mr-1 line-through">${price}</span>
      <span className="font-semibold">
        ${Number((price * (1 - percentOff)).toFixed(2))}
      </span>
    </>
  ) : (
    <span className="font-semibold">${price}</span>
  );
}
