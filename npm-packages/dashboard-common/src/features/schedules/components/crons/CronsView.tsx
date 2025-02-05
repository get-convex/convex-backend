import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { SchedulingLayout } from "@common/layouts/SchedulingLayout";
import { CronJobsProvider } from "@common/features/schedules/lib/CronsProvider";
import { CronJobsContent } from "@common/features/schedules/components/crons/CronJobsContent";

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
