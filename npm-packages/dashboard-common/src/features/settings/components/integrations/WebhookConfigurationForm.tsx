import * as Yup from "yup";
import { useFormik } from "formik";
import { Infer } from "convex/values";
import { webhookConfig } from "system-udfs/convex/schema";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useCreateWebhookIntegration } from "@common/lib/integrationsApi";
import { Combobox } from "@ui/Combobox";

const webhookValidationSchema = Yup.object().shape({
  url: Yup.string().url().required("URL required"),
  format: Yup.string().oneOf(["json", "jsonl"]).required("Format required"),
  authType: Yup.string().oneOf(["none", "basic"]).required(),
  username: Yup.string().when("authType", {
    is: "basic",
    then: (schema) => schema.required("Username required"),
    otherwise: (schema) => schema.notRequired(),
  }),
  password: Yup.string().when("authType", {
    is: "basic",
    then: (schema) => schema.required("Password required"),
    otherwise: (schema) => schema.notRequired(),
  }),
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
      format: existingIntegration?.format ?? "jsonl",
      authType: existingIntegration?.auth?.type ?? "none",
      username:
        existingIntegration?.auth?.type === "basic"
          ? existingIntegration.auth.username
          : "",
      password: "",
    },
    onSubmit: async (values) => {
      const body: any = {
        url: values.url,
        format: values.format,
      };
      if (values.authType === "basic") {
        body.basicAuth = {
          username: values.username,
          password: values.password,
        };
      }
      await createWebhookIntegration(body);
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
      <Combobox
        label="Format"
        labelHidden={false}
        options={[
          { value: "jsonl", label: "JSONL (one object per line of request)" },
          { value: "json", label: "JSON (one array per request)" },
        ]}
        selectedOption={formState.values.format}
        setSelectedOption={async (v) => {
          await formState.setFieldValue("format", v);
        }}
        allowCustomValue={false}
        disableSearch
        buttonClasses="w-full bg-inherit"
      />
      <Combobox
        label="Authentication"
        labelHidden={false}
        options={[
          { value: "none", label: "None" },
          { value: "basic", label: "Basic" },
        ]}
        selectedOption={formState.values.authType}
        setSelectedOption={async (v) => {
          await formState.setFieldValue("authType", v);
        }}
        allowCustomValue={false}
        disableSearch
        buttonClasses="w-full bg-inherit"
      />
      {formState.values.authType === "basic" && (
        <>
          <TextInput
            value={formState.values.username}
            onChange={formState.handleChange}
            id="username"
            label="Username"
            placeholder="Enter username"
            error={formState.errors.username as string | undefined}
          />
          <TextInput
            value={formState.values.password}
            onChange={formState.handleChange}
            id="password"
            type="password"
            label="Password"
            placeholder="Enter password"
            error={formState.errors.password as string | undefined}
          />
        </>
      )}
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
