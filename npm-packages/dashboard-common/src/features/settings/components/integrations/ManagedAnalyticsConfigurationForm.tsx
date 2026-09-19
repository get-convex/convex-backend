import { Button } from "@ui/Button";
import { useFormik } from "formik";
import { SyncPeriod } from "system-udfs/convex/_system/frontend/common";
import { AnalyticsIntegration } from "@common/lib/integrationHelpers";
import {
  useCreateLogStream,
  useUpdateLogStream,
} from "@common/lib/integrationsApi";
import { toast } from "@common/lib/utils";
import { SyncPeriodSelector } from "./SyncPeriodSelector";

export function ManagedAnalyticsConfigurationForm({
  onClose,
  integration,
  onAddedIntegration,
}: {
  onClose: () => void;
  integration: Extract<AnalyticsIntegration, { kind: "managedAnalytics" }>;
  onAddedIntegration?: () => void;
}) {
  const createLogStream = useCreateLogStream();
  const updateLogStream = useUpdateLogStream();
  const existingConfig = integration.existing?.config ?? null;
  const logStreamId = integration.existing?._id;

  const isNewIntegration = existingConfig === null || !logStreamId;

  const formState = useFormik<{ period: SyncPeriod }>({
    initialValues: {
      period: existingConfig?.period ?? "daily",
    },
    onSubmit: async (values, helpers) => {
      helpers.setStatus(undefined);
      try {
        const args = {
          logStreamType: "managedAnalytics" as const,
          period: values.period,
        };

        if (isNewIntegration) {
          await createLogStream(args);
          onAddedIntegration?.();
          toast("success", "Created Managed Analytics integration");
        } else {
          await updateLogStream(logStreamId, args);
          toast("success", "Updated Managed Analytics integration");
        }
        onClose();
      } catch (e) {
        helpers.setStatus({
          error: e instanceof Error ? e.message : "Failed to save integration.",
        });
      }
    },
  });

  return (
    <form onSubmit={formState.handleSubmit} className="flex flex-col gap-3">
      <div className="max-w-prose text-xs text-pretty text-content-secondary">
        Convex keeps the mirror in a bucket we manage, so there is nothing to
        provision on your side. All tables in all components are mirrored.
      </div>
      <SyncPeriodSelector
        value={formState.values.period}
        onChange={async (period) => {
          await formState.setFieldValue("period", period);
        }}
      />
      <div className="flex items-center justify-end gap-3">
        {formState.status?.error && (
          <p className="text-sm text-content-errorSecondary" role="alert">
            {formState.status.error}
          </p>
        )}
        <Button
          variant="primary"
          type="submit"
          aria-label="save"
          disabled={
            (!isNewIntegration && !formState.dirty) || formState.isSubmitting
          }
          loading={formState.isSubmitting}
        >
          Save
        </Button>
      </div>
    </form>
  );
}
