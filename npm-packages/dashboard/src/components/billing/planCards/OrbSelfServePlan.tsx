import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { Button } from "@ui/Button";
import { OrbSubscriptionResponse, PlanResponse, Team } from "generatedApi";
import { useRouter } from "next/router";
import { useChangeSubscription } from "api/billing";
import { useState } from "react";
import {
  DowngradePlanDialog,
  UpgradePlanDialog,
} from "components/billing/planCards/ChangePlanDialogs";
import { SelfServePlan } from "./SelfServePlan";

export function OrbSelfServePlan({
  orbSub,
  plan,
  team,
}: {
  orbSub?: OrbSubscriptionResponse;
  plan: PlanResponse;
  team: Team;
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

  return (
    <>
      <SelfServePlan
        currentPlan={orbSub?.plan.id}
        plan={plan}
        percentOff={0}
        action={
          orbSub?.plan.planType === plan.planType ||
          orbSub?.plan.id === plan.id ? (
            <p className="flex h-[2.125rem] items-center font-semibold">
              Current Plan
            </p>
          ) : orbSub ? (
            <Button
              tip={
                !hasAdminPermissions &&
                "You do not have permission to modify the team subscription."
              }
              onClick={() => {
                setIsChangingPlan(true);
              }}
              disabled={!hasAdminPermissions}
              variant={isDowngrade ? "neutral" : "primary"}
            >
              {isDowngrade
                ? `Downgrade to ${plan.name}`
                : `Upgrade to ${plan.name}`}
            </Button>
          ) : (
            <Button
              onClick={() => upgrade()}
              disabled={!hasAdminPermissions}
              variant={
                plan.planType === "CONVEX_PROFESSIONAL" ? "primary" : "neutral"
              }
              tip={
                !hasAdminPermissions
                  ? "You do not have permission to modify the team subscription."
                  : undefined
              }
            >
              Upgrade to {plan.name}
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
            onConfirm={() => changePlan({ newPlanId: plan.id })}
            newPlan={plan}
            team={team}
          />
        ))}
    </>
  );
}
