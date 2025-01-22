import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { CronJobsContent } from "components/scheduling/crons/CronJobsContent";
import { CronJobsProvider } from "data/Functions/CronsProvider";
import { SchedulingLayout } from "layouts/SchedulingLayout";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";

export { getServerSideProps } from "lib/ssr";

function CronsPage() {
  return (
    <SchedulingLayout>
      <DeploymentPageTitle title="Cron Jobs" />
      <CronJobsProvider>
        <CronJobsContent />
      </CronJobsProvider>
    </SchedulingLayout>
  );
}

export default withAuthenticatedPage(CronsPage);
