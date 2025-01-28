import { DeploymentPageTitle } from "../../../../elements/DeploymentPageTitle";
import { SchedulingLayout } from "../../../../layouts/SchedulingLayout";
import { CronJobsProvider } from "../../lib/CronsProvider";
import { CronJobsContent } from "./CronJobsContent";

export function CronsView() {
  return (
    <SchedulingLayout>
      <DeploymentPageTitle title="Cron Jobs" />
      <CronJobsProvider>
        <CronJobsContent />
      </CronJobsProvider>
    </SchedulingLayout>
  );
}
