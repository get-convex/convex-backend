import { integrationUsingLegacyFormat } from "@common/lib/integrationHelpers";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { TextInput } from "@ui/TextInput";
import { Infer } from "convex/values";
import { useFormik } from "formik";
import { useCreateSentryIntegration } from "@common/lib/integrationsApi";
import Link from "next/link";
import { sentryConfig } from "system-udfs/convex/schema";
import * as Yup from "yup";

const sentryValidationSchema = Yup.object().shape({
  dsn: Yup.string().url().required("Sentry DSN is required"),
  tags: Yup.string()
    .test("is-valid-json", "Tags must be a valid JSON object", (value, ctx) => {
      if (!value) return true; // Allow empty value
      try {
        const parsed = JSON.parse(value);
        return (
          typeof parsed === "object" &&
          parsed !== null &&
          !Array.isArray(parsed)
        );
      } catch (e) {
        return ctx.createError({
          message: `Tags must be a valid JSON object: ${e}`,
        });
      }
    })
    .nullable(),
});

export function SentryConfigurationForm({
  onClose,
  existingConfig,
}: {
  onClose: () => void;
  existingConfig: Infer<typeof sentryConfig> | null;
}) {
  const createSentryIntegration = useCreateSentryIntegration();
  const isUsingLegacyFormat = integrationUsingLegacyFormat(existingConfig);

  const formState = useFormik<{
    dsn: string;
    tags: string | undefined;
    version: "1" | "2";
  }>({
    initialValues: {
      dsn: existingConfig?.dsn ?? "",
      tags: existingConfig?.tags
        ? JSON.stringify(existingConfig.tags)
        : undefined,
      version: existingConfig !== null ? (existingConfig.version ?? "1") : "2",
    },
    onSubmit: async (values) => {
      await createSentryIntegration(
        values.dsn,
        values.tags ? JSON.parse(values.tags) : undefined,
        values.version,
      );
      onClose();
    },
    validationSchema: sentryValidationSchema,
  });

  return (
    <form onSubmit={formState.handleSubmit} className="flex flex-col gap-3">
      {isUsingLegacyFormat && (
        <>
          <div className="flex flex-col gap-1">
            Event Format
            <div className="text-xs text-content-secondary">
              The current version uses the <code>stacktrace</code> instead of
              the <code>value</code> field to capture the stacktrace, enabling
              better Sentry grouping and source code integrations.
            </div>
          </div>
          <Combobox
            label="Select event format"
            options={[
              { value: "1", label: "Legacy" },
              { value: "2", label: "Current" },
            ]}
            selectedOption={formState.values.version}
            setSelectedOption={async (v) => {
              await formState.setFieldValue("version", v);
            }}
            disableSearch
            allowCustomValue={false}
            buttonClasses="w-full bg-inherit"
          />
        </>
      )}
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
                className="text-content-link"
                target="_blank"
              >
                Learn more
              </Link>
            </div>
          </div>
        }
      />
      <TextInput
        value={formState.values.tags}
        onChange={formState.handleChange}
        label="Tags"
        placeholder='{"key": "value"}'
        id="tags"
        error={formState.errors.tags}
        description="Tags to add to all events routed to Sentry. Use JSON format."
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
