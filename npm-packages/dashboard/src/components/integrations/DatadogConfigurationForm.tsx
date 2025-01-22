import Link from "next/link";
import * as Yup from "yup";
import { useFormik } from "formik";
import { DatadogSiteLocation } from "system-udfs/convex/_system/frontend/common";
import { Infer } from "convex/values";
import { datadogConfig } from "system-udfs/convex/schema";
import { EyeNoneIcon, EyeOpenIcon } from "@radix-ui/react-icons";
import { useCurrentProject } from "api/projects";
import { useState } from "react";
import { TextInput } from "elements/TextInput";
import {
  Button,
  Combobox,
  Option,
  integrationUsingLegacyFormat,
} from "dashboard-common";
import { useCreateDatadogSink } from "hooks/deploymentApi";

const siteLocationOptions: Option<DatadogSiteLocation>[] = [
  { value: "US1", label: "US1" },
  { value: "US3", label: "US3" },
  { value: "US5", label: "US5" },
  { value: "EU", label: "EU1" },
  { value: "US1_FED", label: "US1-FED" },
  { value: "AP1", label: "AP1" },
];

const datadogValidationSchema = Yup.object().shape({
  siteLocation: Yup.string().required("Site location is required"),
  ddApiKey: Yup.string().required("API key is required"),
  ddTags: Yup.string(),
});

export function DatadogConfigurationForm({
  existingConfig,
  onClose,
}: {
  onClose: () => void;
  existingConfig: Infer<typeof datadogConfig> | null;
}) {
  const isUsingLegacyFormat = integrationUsingLegacyFormat(existingConfig);

  const createDatadogSink = useCreateDatadogSink();
  const project = useCurrentProject();

  const [showApiKey, setShowApiKey] = useState(false);

  const formState = useFormik<{
    siteLocation: DatadogSiteLocation;
    ddApiKey: string;
    ddTags: string;
    service: string | null;
    version: "1" | "2";
  }>({
    initialValues: {
      siteLocation: existingConfig?.siteLocation ?? "US1",
      ddApiKey: existingConfig?.ddApiKey ?? "",
      ddTags: existingConfig?.ddTags.join(",") ?? "",
      service:
        (existingConfig ? existingConfig.service : project?.name) ?? null,
      version: existingConfig !== null ? (existingConfig.version ?? "1") : "2",
    },
    onSubmit: async (values) => {
      await createDatadogSink(
        values.siteLocation,
        values.ddApiKey,
        values.ddTags.split(",").filter((v) => v !== ""),
        values.service,
        values.version,
      );
      onClose();
    },
    validationSchema: datadogValidationSchema,
  });
  return (
    <form onSubmit={formState.handleSubmit} className="flex flex-col gap-3">
      {isUsingLegacyFormat && (
        <>
          <div className="flex flex-col gap-1">
            Event Format
            <div className="text-xs text-content-secondary">
              Format for events sent in this stream.{" "}
              <Link
                href="https://docs.convex.dev/production/integrations/log-streams/legacy-event-schema"
                className="text-content-link dark:underline"
                target="_blank"
              >
                Learn more
              </Link>
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
            allowCustomValue={false}
            buttonClasses="w-full bg-inherit"
          />
        </>
      )}
      <div className="flex flex-col gap-1">
        <Combobox
          label="Site Location"
          labelHidden={false}
          options={siteLocationOptions}
          selectedOption={formState.values.siteLocation}
          setSelectedOption={async (loc) => {
            await formState.setFieldValue("siteLocation", loc);
          }}
          allowCustomValue={false}
          buttonClasses="w-full bg-inherit"
        />
        <div className="max-w-prose text-xs text-content-secondary">
          Location of your Datadog deployment.{" "}
          <Link
            href="https://docs.datadoghq.com/getting_started/site/"
            className="text-content-link dark:underline"
            target="_blank"
          >
            Learn more
          </Link>
        </div>
      </div>
      <TextInput
        label="API Key"
        value={formState.values.ddApiKey}
        type={showApiKey ? "text" : "password"}
        onChange={formState.handleChange}
        id="ddApiKey"
        placeholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        action={() => setShowApiKey(!showApiKey)}
        Icon={showApiKey ? EyeNoneIcon : EyeOpenIcon}
        error={formState.errors.ddApiKey}
        description="API key is used to authenticate with Datadog."
      />
      <TextInput
        value={formState.values.service || undefined}
        onChange={formState.handleChange}
        placeholder={project?.name}
        label="Service"
        id="service"
        error={formState.errors.service}
        description="Service name used as a special tag in Datadog."
      />
      <TextInput
        value={formState.values.ddTags}
        onChange={formState.handleChange}
        id="ddTags"
        label="Tags"
        error={formState.errors.ddTags}
        description={
          <div className="text-xs text-content-secondary">
            Optional comma-separated list of tags. These are sent to Datadog in
            each log event via the <code>ddtags</code> field.{" "}
            <Link
              href="https://docs.datadoghq.com/getting_started/tagging/"
              className="text-content-link dark:underline"
              target="_blank"
            >
              Learn more
            </Link>
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
