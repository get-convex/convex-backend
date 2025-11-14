import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { Button } from "@ui/Button";
import {
  OrbSubscriptionResponse,
  PlanResponse,
  TeamResponse,
} from "generatedApi";
import { useRouter } from "next/router";
import { useChangeSubscription } from "api/billing";
import { useState } from "react";
import {
  DowngradePlanDialog,
  UpgradePlanDialog,
} from "components/billing/planCards/ChangePlanDialogs";
import { planNameMap } from "components/billing/planCards/PlanCard";
import startCase from "lodash/startCase";
import { SelfServePlan } from "./SelfServePlan";

export function OrbSelfServePlan({
  orbSub,
  plan,
  team,
}: {
  orbSub?: OrbSubscriptionResponse;
  plan: PlanResponse;
  team: TeamResponse;
}) {
  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();

  const { push } = useRouter();

  const upgrade = () => {
    void push(
      {
        pathname: "/t/[team]/settings/billing",
        query: {
          team: team.slug,
          upgradePlan: plan.id,
        },
      },
      undefined,
      { shallow: true },
    );
  };

  const isDowngrade =
    orbSub?.plan.planType === "CONVEX_PROFESSIONAL" &&
    plan.planType === "CONVEX_STARTER_PLUS";

  const changePlan = useChangeSubscription(team.id);

  const [isChangingPlan, setIsChangingPlan] = useState(false);

  const newPlanName = plan.planType
    ? planNameMap[plan.planType] || plan.name
    : plan.name;
  const missingRequiredPaymentMethod =
    orbSub &&
    orbSub?.paymentMethod === null &&
    orbSub?.plan.planType === "CONVEX_PROFESSIONAL" &&
    plan.planType === "CONVEX_STARTER_PLUS";

  return (
    <>
      <SelfServePlan
        currentPlan={orbSub?.plan.id}
        plan={plan}
        percentOff={0}
        action={
          orbSub?.plan.planType === plan.planType ||
          orbSub?.plan.id === plan.id ? (
            <p className="flex h-[2.125rem] items-center px-2 font-semibold">
              Current Plan
            </p>
          ) : orbSub ? (
            <Button
              tip={
                !hasAdminPermissions
                  ? "You do not have permission to modify the team subscription."
                  : team.managedBy
                    ? plan.planType === "CONVEX_PROFESSIONAL"
                      ? `Convex Professional is not available for teams managed by ${startCase(team.managedBy)}. You can create a new team to use Convex Professional. Existing projects can be transferred to another team on the Project Settings page.`
                      : `You can manage your subscription in ${startCase(team.managedBy)}.`
                    : missingRequiredPaymentMethod
                      ? "Add a payment method in the settings below to switch to this plan."
                      : undefined
              }
              onClick={() => {
                setIsChangingPlan(true);
              }}
              disabled={
                !hasAdminPermissions ||
                missingRequiredPaymentMethod ||
                !!team.managedBy
              }
              variant={isDowngrade ? "neutral" : "primary"}
            >
              {isDowngrade
                ? `Downgrade to ${newPlanName}`
                : `Upgrade to ${newPlanName}`}
            </Button>
          ) : (
            <Button
              onClick={() => upgrade()}
              disabled={!hasAdminPermissions || !!team.managedBy}
              variant={
                plan.planType === "CONVEX_PROFESSIONAL" ? "primary" : "neutral"
              }
              tip={
                !hasAdminPermissions
                  ? "You do not have permission to modify the team subscription."
                  : team.managedBy
                    ? plan.planType === "CONVEX_PROFESSIONAL"
                      ? `Convex Professional is not available for teams managed by ${startCase(team.managedBy)}. You can create a new team to use Convex Professional. Existing projects can be transferred to another team on the Project Settings page.`
                      : `You can manage your subscription in ${startCase(team.managedBy)}.`
                    : undefined
              }
            >
              Upgrade to {newPlanName}
            </Button>
          )
        }
      />
      {isChangingPlan &&
        (isDowngrade ? (
          <DowngradePlanDialog
            onClose={() => setIsChangingPlan(false)}
            onConfirm={() => changePlan({ newPlanId: plan.id })}
            newPlan={plan}
            team={team}
          />
        ) : (
          <UpgradePlanDialog
            onClose={() => setIsChangingPlan(false)}
            onConfirm={(newPlanId) => changePlan({ newPlanId })}
            newPlan={plan}
            team={team}
          />
        ))}
    </>
  );
}
