import * as Yup from "yup";
import { useFormik } from "formik";
import { Infer } from "convex/values";
import { webhookConfig } from "system-udfs/convex/schema";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useCreateWebhookIntegration } from "@common/lib/integrationsApi";

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
  const createWebhookIntegration = useCreateWebhookIntegration();

  const formState = useFormik({
    initialValues: {
      url: existingIntegration?.url ?? "",
    },
    onSubmit: async (values) => {
      await createWebhookIntegration(values.url);
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
        placeholder="Enter a URL to send logs to"
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
