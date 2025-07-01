import Link from "next/link";
import * as Yup from "yup";
import { FieldArray, FormikProvider, getIn, useFormik } from "formik";
import {
  EyeNoneIcon,
  EyeOpenIcon,
  PlusIcon,
  TrashIcon,
} from "@radix-ui/react-icons";
import { AxiomConfig } from "system-udfs/convex/_system/frontend/common";
import { axiomConfig } from "system-udfs/convex/schema";
import { Infer } from "convex/values";
import { Combobox } from "@ui/Combobox";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { integrationUsingLegacyFormat } from "@common/lib/integrationHelpers";
import { useState } from "react";
import { useCreateAxiomIntegration } from "@common/lib/integrationsApi";

const axiomValidationSchema = Yup.object().shape({
  datasetName: Yup.string().required("Dataset name is required"),
  apiKey: Yup.string().required("API key is required"),
  attributes: Yup.array().of(
    Yup.object().shape({
      key: Yup.string().required("Name is required"),
      value: Yup.string().required("Value is required"),
    }),
  ),
});

type Unpacked<T> = T extends (infer U)[] ? U : never;

export function AxiomConfigurationForm({
  onClose,
  existingConfig,
}: {
  onClose: () => void;
  existingConfig: Infer<typeof axiomConfig> | null;
}) {
  const isUsingLegacyFormat = integrationUsingLegacyFormat(existingConfig);
  const createAxiomIntegration = useCreateAxiomIntegration();

  const formState = useFormik<{
    datasetName: string;
    apiKey: string;
    attributes: Unpacked<AxiomConfig["attributes"]>[];
    version: "1" | "2";
  }>({
    initialValues: {
      datasetName: existingConfig?.datasetName ?? "",
      apiKey: existingConfig?.apiKey ?? "",
      attributes: existingConfig?.attributes ?? [],
      version: existingConfig !== null ? (existingConfig.version ?? "1") : "2",
    },
    onSubmit: async (values) => {
      await createAxiomIntegration(
        values.datasetName,
        values.apiKey,
        values.attributes,
        values.version,
      );
      onClose();
    },
    validationSchema: axiomValidationSchema,
  });

  const [showApiKey, setShowApiKey] = useState(false);

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
                className="text-content-link"
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
      <TextInput
        value={formState.values.datasetName}
        onChange={formState.handleChange}
        label="Dataset Name"
        id="datasetName"
        error={formState.errors.datasetName}
        description="Name of the dataset in Axiom. This is where the logs will be sent."
      />
      <TextInput
        label="API Key"
        value={formState.values.apiKey}
        type={showApiKey ? "text" : "password"}
        onChange={formState.handleChange}
        className="max-w-full"
        id="apiKey"
        placeholder="xxxx-xxxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
        action={() => setShowApiKey(!showApiKey)}
        Icon={showApiKey ? EyeNoneIcon : EyeOpenIcon}
        error={formState.errors.apiKey}
        description="API key is used to authenticate with Axiom."
      />
      <div className="flex flex-col gap-1">
        Attributes
        <div className="text-xs text-content-secondary">
          Optional list of attributes. These are extra fields and values sent to
          Axiom in each log event.{" "}
          <Link
            href="https://axiom.co/docs/send-data/ingest#ingest-api"
            className="text-content-link"
            target="_blank"
          >
            Learn more
          </Link>
        </div>
      </div>
      <FormikProvider value={formState}>
        <FieldArray
          name="attributes"
          render={(arrayHelpers) => (
            <>
              {formState.values.attributes.map(({ key, value }, idx) => (
                <div
                  className="flex flex-row items-start justify-between gap-1"
                  key={idx}
                >
                  <TextInput
                    className="w-full"
                    value={key}
                    labelHidden
                    placeholder="Name"
                    id={`attributes[${idx}].key`}
                    onChange={formState.handleChange}
                    error={getIn(formState.errors, `attributes[${idx}].key`)}
                  />
                  <TextInput
                    className="w-full"
                    labelHidden
                    value={value}
                    placeholder="Value"
                    id={`attributes[${idx}].value`}
                    onChange={formState.handleChange}
                    error={getIn(formState.errors, `attributes[${idx}].value`)}
                  />
                  <Button
                    onClick={() => arrayHelpers.remove(idx)}
                    variant="danger"
                    size="sm"
                    inline
                    className="mt-1 h-fit"
                    icon={<TrashIcon />}
                  />
                </div>
              ))}
              <Button
                variant="neutral"
                className="ml-auto w-fit"
                onClick={() => arrayHelpers.push({ key: "", value: "" })}
              >
                <PlusIcon />
                Add attribute
              </Button>
            </>
          )}
        />
      </FormikProvider>
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
