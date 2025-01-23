import { Button, TextInput } from "dashboard-common";
import { Infer } from "convex/values";
import { useFormik } from "formik";
import { useCreateSentrySink } from "hooks/deploymentApi";
import Link from "next/link";
import { sentryConfig } from "system-udfs/convex/schema";
import * as Yup from "yup";

const sentryValidationSchema = Yup.object().shape({
  dsn: Yup.string().url().required("Sentry DSN is required"),
});

export function SentryConfigurationForm({
  onClose,
  existingConfig,
}: {
  onClose: () => void;
  existingConfig: Infer<typeof sentryConfig> | null;
}) {
  const createSentrySink = useCreateSentrySink();

  const formState = useFormik<{
    dsn: string;
  }>({
    initialValues: {
      dsn: existingConfig?.dsn ?? "",
    },
    onSubmit: async (values) => {
      await createSentrySink(values.dsn);
      onClose();
    },
    validationSchema: sentryValidationSchema,
  });

  return (
    <form onSubmit={formState.handleSubmit} className="flex flex-col gap-3">
      <TextInput
        value={formState.values.dsn}
        onChange={formState.handleChange}
        label="Sentry Data Source Name (DSN)"
        placeholder="https://xxxx@xxxx.ingest.sentry.io/xxxx"
        id="dsn"
        error={formState.errors.dsn}
        description={
          <div className="flex flex-col gap-1">
            <div className="text-xs text-content-secondary">
              Sentry Data Source Name (DSN) to route exceptions to.{" "}
              <Link
                href="https://docs.sentry.io/product/sentry-basics/concepts/dsn-explainer/"
                className="text-content-link dark:underline"
                target="_blank"
              >
                Learn more
              </Link>
            </div>
          </div>
        }
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
