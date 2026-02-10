import { ExceptionReportingIntegration } from "@common/lib/integrationHelpers";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useFormik } from "formik";
import {
  useCreateLogStream,
  useUpdateLogStream,
} from "@common/lib/integrationsApi";
import { toast } from "@common/lib/utils";
import * as Yup from "yup";

const validationSchema = Yup.object().shape({
  apiKey: Yup.string().required("PostHog project API key is required"),
  host: Yup.string().url("Must be a valid URL").nullable(),
});

export function PostHogErrorTrackingConfigurationForm({
  onClose,
  integration,
  onAddedIntegration,
}: {
  onClose: () => void;
  integration: Extract<
    ExceptionReportingIntegration,
    { kind: "postHogErrorTracking" }
  >;
  onAddedIntegration?: () => void;
}) {
  const createLogStream = useCreateLogStream();
  const updateLogStream = useUpdateLogStream();
  const existingConfig = integration.existing?.config ?? null;
  const logStreamId = integration.existing?._id;

  const isNewIntegration = existingConfig === null || !logStreamId;

  const formState = useFormik<{
    apiKey: string;
    host: string;
  }>({
    initialValues: {
      apiKey: existingConfig?.apiKey ?? "",
      host: existingConfig?.host ?? "",
    },
    onSubmit: async (values) => {
      const args = {
        logStreamType: "postHogErrorTracking" as const,
        apiKey: values.apiKey,
        host: values.host || undefined,
      };

      if (isNewIntegration) {
        await createLogStream(args);
        onAddedIntegration?.();
        toast("success", "Created PostHog Error Tracking integration");
      } else {
        await updateLogStream(logStreamId, args);
        toast("success", "Updated PostHog Error Tracking integration");
      }
      onClose();
    },
    validationSchema,
  });

  return (
    <form onSubmit={formState.handleSubmit} className="flex flex-col gap-3">
      <TextInput
        value={formState.values.apiKey}
        onChange={formState.handleChange}
        label="Project API Key"
        placeholder="phc_..."
        id="apiKey"
        error={formState.errors.apiKey}
        description="Your PostHog project API key. Found in PostHog under Settings > Project > Project API Key."
      />
      <TextInput
        value={formState.values.host}
        onChange={formState.handleChange}
        label="Host (optional)"
        placeholder="https://us.i.posthog.com"
        id="host"
        error={formState.errors.host}
        description="PostHog instance URL. Defaults to US Cloud. Use https://eu.i.posthog.com for EU Cloud, or your self-hosted URL."
      />
      <div className="flex justify-end">
        <Button
          variant="primary"
          type="submit"
          aria-label="save"
          disabled={!formState.dirty}
        >
          Save
        </Button>
      </div>
    </form>
  );
}
