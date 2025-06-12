import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { Button } from "@ui/Button";
import { useSupportFormOpen } from "elements/SupportWidget";
import { OrbSubscriptionResponse, PlanResponse, Team } from "generatedApi";
import { useRouter } from "next/router";
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

  const [, openSupportForm] = useSupportFormOpen();

  return (
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
              openSupportForm({
                defaultSubject: `Switch to ${plan.name} Plan`,
                defaultMessage: `Team Slug: ${team.slug}`,
              });
            }}
            disabled={!hasAdminPermissions}
          >
            Contact Us
          </Button>
        ) : (
          <Button
            onClick={() => upgrade()}
            disabled={!hasAdminPermissions}
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
  );
}
