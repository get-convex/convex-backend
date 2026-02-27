import { useListPlans } from "api/billing";
import { Loading } from "@ui/Loading";
import { OrbSubscriptionResponse, TeamResponse } from "generatedApi";
import { OrbSelfServePlan } from "./planCards/OrbSelfServePlan";
import { FreePlan } from "./planCards/FreePlan";
import { BusinessPlan } from "./planCards/BusinessPlan";

export function Plans({
  team,
  hasAdminPermissions,
  subscription,
}: {
  team: TeamResponse;
  hasAdminPermissions: boolean;
  subscription?: OrbSubscriptionResponse;
}) {
  const orbPlans = useListPlans(team.id);

  return orbPlans.plans !== undefined ? (
    <div className="scrollbar flex gap-3 overflow-x-auto pb-2">
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
      <BusinessPlan subscription={subscription} />
    </div>
  ) : (
    <Loading className="h-[7.75rem] w-full" fullHeight={false} />
  );
}
