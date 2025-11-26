import * as Yup from "yup";
import { useFormik } from "formik";
import { Infer } from "convex/values";
import { webhookConfig } from "system-udfs/convex/schema";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import {
  useCreateWebhookIntegration,
  useRegenerateWebhookSecret,
} from "@common/lib/integrationsApi";
import { Combobox } from "@ui/Combobox";
import {
  ClipboardCopyIcon,
  EyeNoneIcon,
  EyeOpenIcon,
} from "@radix-ui/react-icons";
import { useState } from "react";
import { copyTextToClipboard, toast } from "@common/lib/utils";
import { Snippet } from "@common/elements/Snippet";

const webhookValidationSchema = Yup.object().shape({
  url: Yup.string().url().required("URL required"),
  format: Yup.string().oneOf(["json", "jsonl"]).required("Format required"),
});

function HmacSecretDisplay({
  hmacSecret,
  initialShowSecret = false,
}: {
  hmacSecret: string;
  initialShowSecret?: boolean;
}) {
  const [showSecret, setShowSecret] = useState(initialShowSecret);

  const maskedSecret = "••••••••••••••••••••••••••••••••";
  const displayValue = showSecret ? hmacSecret : maskedSecret;

  return (
    <>
      <span className="text-left text-sm text-content-primary">
        HMAC Secret
      </span>
      <div className="flex flex-col gap-2">
        <div className="flex flex-1 flex-row items-center gap-2">
          <Snippet value={displayValue} monospace className="flex-1" />
          <Button
            tip={showSecret ? "Hide secret" : "Show secret"}
            type="button"
            onClick={() => setShowSecret(!showSecret)}
            size="xs"
            variant="neutral"
            icon={showSecret ? <EyeNoneIcon /> : <EyeOpenIcon />}
          />
          <Button
            tip="Copy value"
            type="button"
            onClick={async () => {
              await copyTextToClipboard(hmacSecret);
              toast("success", "HMAC secret copied to clipboard");
            }}
            size="xs"
            variant="neutral"
            icon={<ClipboardCopyIcon />}
          />
        </div>
        <p className="text-sm text-content-secondary">
          Use this secret to verify webhook signatures.{" "}
          <a
            href="https://docs.convex.dev/production/integrations/log-streams/#webhook"
            target="_blank"
            rel="noopener noreferrer"
            className="text-content-link hover:underline"
          >
            Learn more about securing webhooks.
          </a>
        </p>
      </div>
    </>
  );
}

export function WebhookConfigurationForm({
  onClose,
  existingIntegration,
}: {
  onClose: () => void;
  existingIntegration: Infer<typeof webhookConfig> | null;
}) {
  const createWebhookIntegration = useCreateWebhookIntegration();
  const regenerateWebhookSecret = useRegenerateWebhookSecret();
  const [showSecretRevealScreen, setShowSecretRevealScreen] = useState(false);
  const isHmacSecretLoading =
    showSecretRevealScreen && !existingIntegration?.hmacSecret;

  const isNewIntegration = existingIntegration === null;

  const formState = useFormik({
    initialValues: {
      url: existingIntegration?.url ?? "",
      format: existingIntegration?.format ?? "jsonl",
    },
    onSubmit: async (values) => {
      await createWebhookIntegration(values.url, values.format);

      // If this is a new integration, wait for the secret to be generated
      if (isNewIntegration) {
        setShowSecretRevealScreen(true);
      } else {
        onClose();
      }
    },
    validationSchema: webhookValidationSchema,
  });

  // Show the secret reveal screen
  if (showSecretRevealScreen && existingIntegration?.hmacSecret) {
    return (
      <div className="flex flex-col gap-4">
        <HmacSecretDisplay
          hmacSecret={existingIntegration.hmacSecret}
          initialShowSecret
        />
        <Button className="ml-auto" variant="primary" onClick={onClose}>
          Done
        </Button>
      </div>
    );
  }

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
      {existingIntegration?.hmacSecret && (
        <>
          <HmacSecretDisplay hmacSecret={existingIntegration?.hmacSecret} />
          <div>
            <Button type="button" onClick={regenerateWebhookSecret}>
              Regenerate secret
            </Button>
          </div>
        </>
      )}
      <div className="flex justify-end">
        <Button
          variant="primary"
          type="submit"
          aria-label="save"
          disabled={!formState.dirty || isHmacSecretLoading}
          loading={isHmacSecretLoading}
        >
          Save
        </Button>
      </div>
    </form>
  );
}
