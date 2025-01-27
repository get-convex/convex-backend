import { SchedulingLayout } from "../../../../layouts/SchedulingLayout";
import { CronJobsProvider } from "../../lib/CronsProvider";
import { CronJobsContent } from "./CronJobsContent";

export function CronsView() {
  return (
    <SchedulingLayout>
      <CronJobsProvider>
        <CronJobsContent />
      </CronJobsProvider>
    </SchedulingLayout>
  );
}
