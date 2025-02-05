import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { SchedulingLayout } from "@common/layouts/SchedulingLayout";
import { useCurrentOpenFunction } from "@common/lib/functions/FunctionsProvider";
import { ScheduledFunctionsContent } from "@common/features/schedules/components/ScheduledFunctionsContent";

export function ScheduledFunctionsView() {
  const currentOpenFunction = useCurrentOpenFunction();
  return (
    <SchedulingLayout>
      <DeploymentPageTitle title="Scheduled Functions" />
      <ScheduledFunctionsContent
        currentOpenFunction={currentOpenFunction ?? undefined}
        // Important! This key is used to reset the state of the component when the currentOpenFunction changes
        key={currentOpenFunction ? JSON.stringify(currentOpenFunction) : "all"}
      />
    </SchedulingLayout>
  );
}
