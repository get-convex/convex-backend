import { useContext, useEffect } from "react";
import { useRouter } from "next/router";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import {
  DeploymentInfoContext,
  PermissionsContext,
} from "@common/lib/deploymentContext";
import {
  NoPermissionMessage,
  PermissionDeniedTip,
} from "@common/elements/NoPermissionMessage";
import { Sheet } from "@ui/Sheet";
import {
  UsageLimits,
  computeUnbilledMetrics,
} from "@common/features/settings/components/UsageLimits";
import {
  useUsageLimits,
  useCreateUsageLimit,
  useUpdateUsageLimit,
  useDeleteUsageLimit,
} from "@common/features/settings/lib/api";

export function UsageLimitsView() {
  const { useIsOperationAllowed } = useContext(PermissionsContext);
  const canView = useIsOperationAllowed("ViewUsageLimits");
  const canWrite = useIsOperationAllowed("WriteUsageLimits");

  // Usage limits is feature-flagged; if it's off, don't render it even when
  // reached by a direct URL — send the user back to deployment settings.
  const router = useRouter();
  const { usageLimitsEnabled, deploymentsURI } = useContext(
    DeploymentInfoContext,
  );
  useEffect(() => {
    if (!usageLimitsEnabled) {
      void router.replace(`${deploymentsURI}/settings`);
    }
  }, [usageLimitsEnabled, deploymentsURI, router]);
  if (!usageLimitsEnabled) {
    return null;
  }

  return (
    <DeploymentSettingsLayout page="usage-limits">
      {canView ? (
        <UsageLimitsContent canWrite={canWrite} />
      ) : (
        <Sheet className="max-w-3xl py-12">
          <NoPermissionMessage
            message="You do not have permission to view usage limits."
            missingPermission="deployment:usageLimits:view"
          />
        </Sheet>
      )}
    </DeploymentSettingsLayout>
  );
}

function UsageLimitsContent({ canWrite }: { canWrite: boolean }) {
  const { useCurrentDeployment, useCurrentTeam, useTeamPlanType } = useContext(
    DeploymentInfoContext,
  );
  const deployment = useCurrentDeployment();
  const team = useCurrentTeam();
  const planType = useTeamPlanType(team?.id ?? null);
  // Business and Enterprise share the "CONVEX_BUSINESS" plan type.
  const isBusinessPlan = planType === "CONVEX_BUSINESS";
  // Dedicated (DXXXX) deployment classes start with "d" (e.g. "d1024").
  const isDedicated =
    deployment?.kind === "cloud" && deployment.class.startsWith("d");
  // Billing tiers are a Convex Cloud concept; self-hosted deployments have no
  // plan or billing, so the "unbilled metric" callouts don't apply there.
  const unbilledMetrics =
    deployment?.kind === "cloud"
      ? computeUnbilledMetrics({ isBusinessPlan, isDedicated })
      : {};

  const { usageLimits, isLoading } = useUsageLimits();
  const createUsageLimit = useCreateUsageLimit();
  const updateUsageLimit = useUpdateUsageLimit();
  const deleteUsageLimit = useDeleteUsageLimit();

  return (
    <UsageLimits
      usageLimits={usageLimits ?? []}
      isLoading={isLoading}
      canWrite={canWrite}
      writePermissionTip={
        <PermissionDeniedTip
          message="You do not have permission to modify usage limits."
          action="deployment:usageLimits:write"
        />
      }
      unbilledMetrics={unbilledMetrics}
      deploymentType={deployment?.deploymentType}
      onCreate={createUsageLimit}
      onUpdate={updateUsageLimit}
      onDelete={deleteUsageLimit}
    />
  );
}
