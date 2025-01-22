import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ScheduledFunctionsContent } from "components/scheduling/ScheduledFunctionsContent";
import { SchedulingLayout } from "layouts/SchedulingLayout";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { useCurrentOpenFunction } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

function FunctionsPage() {
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

export default withAuthenticatedPage(FunctionsPage);
