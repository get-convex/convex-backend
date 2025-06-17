import { useListPlans } from "api/billing";
import { Loading } from "@ui/Loading";
import { OrbSubscriptionResponse, Team } from "generatedApi";
import classNames from "classnames";
import { OrbSelfServePlan } from "./planCards/OrbSelfServePlan";
import { FreePlan } from "./planCards/FreePlan";

export function Plans({
  team,
  hasAdminPermissions,
  subscription,
}: {
  team: Team;
  hasAdminPermissions: boolean;
  subscription?: OrbSubscriptionResponse;
}) {
  const orbPlans = useListPlans(team.id);

  return orbPlans.plans !== undefined ? (
    <div
      className={classNames(
        "grid grid-cols-1 gap-6",
        // TODO: Remove when we always have > 1 plan
        orbPlans.plans.length > 1 ? "xl:grid-cols-3" : "lg:grid-cols-2",
      )}
    >
      <FreePlan
        hasAdminPermissions={hasAdminPermissions}
        subscription={subscription}
        team={team}
      />
      {orbPlans.plans.map((plan, idx) => (
        <OrbSelfServePlan
          key={idx}
          orbSub={subscription}
          plan={plan}
          team={team}
        />
      ))}
    </div>
  ) : (
    <Loading className="h-48 w-full" fullHeight={false} />
  );
}
