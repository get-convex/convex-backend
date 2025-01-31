import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { SchedulingLayout } from "layouts/SchedulingLayout";
import { useCurrentOpenFunction } from "lib/functions/FunctionsProvider";
import { ScheduledFunctionsContent } from "features/schedules/components/ScheduledFunctionsContent";

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
