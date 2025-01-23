import { useListPlans } from "api/billing";
import { Loading } from "dashboard-common";
import { OrbSubscriptionResponse, Team } from "generatedApi";
import { BusinessPlan } from "./planCards/BusinessPlan";
import { OrbSelfServePlan } from "./planCards/OrbSelfServePlan";
import { StarterPlan } from "./planCards/StarterPlan";

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
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <StarterPlan
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
      <BusinessPlan hasAdminPermissions={hasAdminPermissions} />
    </div>
  ) : (
    <Loading className="h-48 w-full" fullHeight={false} />
  );
}
