import * as Yup from "yup";
import { useFormik } from "formik";
import { Infer } from "convex/values";
import { webhookConfig } from "system-udfs/convex/schema";
import { Button } from "dashboard-common";
import { TextInput } from "elements/TextInput";
import { useCreateWebhookSink } from "../../hooks/deploymentApi";

const webhookValidationSchema = Yup.object().shape({
  url: Yup.string().url().required("URL required"),
});

export function WebhookConfigurationForm({
  onClose,
  existingIntegration,
}: {
  onClose: () => void;
  existingIntegration: Infer<typeof webhookConfig> | null;
}) {
  const createWebhookSink = useCreateWebhookSink();

  const formState = useFormik({
    initialValues: {
      url: existingIntegration?.url ?? "",
    },
    onSubmit: async (values) => {
      await createWebhookSink(values.url);
      onClose();
    },
    validationSchema: webhookValidationSchema,
  });

  return (
    <form onSubmit={formState.handleSubmit} className="flex flex-col gap-3">
      <TextInput
        value={formState.values.url}
        onChange={formState.handleChange}
        id="url"
        label="URL"
        error={formState.errors.url}
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
