import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { SchedulingLayout } from "layouts/SchedulingLayout";
import { CronJobsProvider } from "features/schedules/lib/CronsProvider";
import { CronJobsContent } from "features/schedules/components/crons/CronJobsContent";

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
