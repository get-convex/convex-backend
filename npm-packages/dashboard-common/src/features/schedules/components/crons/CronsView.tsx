import { useContext } from "react";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { SchedulingLayout } from "@common/layouts/SchedulingLayout";
import { CronJobsProvider } from "@common/features/schedules/lib/CronsProvider";
import { CronJobsContent } from "@common/features/schedules/components/crons/CronJobsContent";

export function CronsView() {
  const { useIsOperationAllowed } = useContext(DeploymentInfoContext);
  const canViewData = useIsOperationAllowed("ViewData");

  if (!canViewData) {
    return (
      <>
        <DeploymentPageTitle title="Cron Jobs" />
        <NoPermissionMessage message="You do not have permission to view cron jobs in this deployment." />
      </>
    );
  }

  return (
    <SchedulingLayout>
      <DeploymentPageTitle title="Cron Jobs" />
      <CronJobsProvider>
        <CronJobsContent />
      </CronJobsProvider>
    </SchedulingLayout>
  );
}
