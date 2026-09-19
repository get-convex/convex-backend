import { Button } from "@ui/Button";
import { Link } from "@ui/Link";
import { TextInput } from "@ui/TextInput";
import { EyeNoneIcon, EyeOpenIcon } from "@radix-ui/react-icons";
import { useFormik } from "formik";
import { useMemo, useState } from "react";
import * as Yup from "yup";
import { SyncPeriod } from "system-udfs/convex/_system/frontend/common";
import { AnalyticsIntegration } from "@common/lib/integrationHelpers";
import {
  useCreateLogStream,
  useUpdateLogStream,
} from "@common/lib/integrationsApi";
import { toast } from "@common/lib/utils";
import { SyncPeriodSelector } from "./SyncPeriodSelector";

// Mirrors `validate_s3_bucket` in crates/local_backend/src/log_sinks.rs.
// https://docs.aws.amazon.com/AmazonS3/latest/userguide/bucketnamingrules.html
const bucketSchema = Yup.string()
  .required("Bucket name is required")
  .min(3, "Bucket names are between 3 and 63 characters")
  .max(63, "Bucket names are between 3 and 63 characters")
  .matches(
    /^[a-z0-9][a-z0-9.-]*[a-z0-9]$/,
    "Bucket names may only use lowercase letters, numbers, dots, and hyphens, and must start and end with a letter or number",
  )
  .test(
    "no-adjacent-periods",
    "Bucket names may not contain two adjacent periods",
    (value) => !value?.includes(".."),
  )
  .test(
    "not-ip-address",
    "Bucket names may not be formatted as an IP address",
    (value) => !/^\d{1,3}(\.\d{1,3}){3}$/.test(value ?? ""),
  )
  .test(
    "no-reserved-affix",
    "Bucket names may not start with `xn--` or `sthree-`, or end with `-s3alias` or `--ol-s3`",
    (value) =>
      !["xn--", "sthree-"].some((p) => value?.startsWith(p)) &&
      !["-s3alias", "--ol-s3"].some((suffix) => value?.endsWith(suffix)),
  );

export function S3ExportConfigurationForm({
  onClose,
  integration,
  onAddedIntegration,
}: {
  onClose: () => void;
  integration: Extract<AnalyticsIntegration, { kind: "s3Export" }>;
  onAddedIntegration?: () => void;
}) {
  const createLogStream = useCreateLogStream();
  const updateLogStream = useUpdateLogStream();
  const existingConfig = integration.existing?.config ?? null;
  const logStreamId = integration.existing?._id;

  const isNewIntegration = existingConfig === null || !logStreamId;

  const [showSecretAccessKey, setShowSecretAccessKey] = useState(false);

  const storedAccessKeyId = existingConfig?.accessKeyId;
  const validationSchema = useMemo(
    () =>
      Yup.object().shape({
        bucket: bucketSchema,
        region: Yup.string().required("Region is required"),
        prefix: Yup.string(),
        accessKeyId: Yup.string().required("Access key ID is required"),
        secretAccessKey: isNewIntegration
          ? Yup.string().required("Secret access key is required")
          : // Blank keeps the stored secret, but a different access key ID
            // needs the secret that goes with it.
            Yup.string().test(
              "secret-required-for-new-key",
              "Enter the secret access key for this access key ID",
              (value, ctx) =>
                !!value || ctx.parent.accessKeyId === storedAccessKeyId,
            ),
      }),
    [isNewIntegration, storedAccessKeyId],
  );

  const formState = useFormik<{
    bucket: string;
    region: string;
    prefix: string;
    accessKeyId: string;
    secretAccessKey: string;
    period: SyncPeriod;
  }>({
    initialValues: {
      bucket: existingConfig?.bucket ?? "",
      region: existingConfig?.region ?? "",
      prefix: existingConfig?.prefix ?? "",
      accessKeyId: existingConfig?.accessKeyId ?? "",
      secretAccessKey: "",
      period: existingConfig?.period ?? "daily",
    },
    onSubmit: async (values, helpers) => {
      helpers.setStatus(undefined);
      try {
        const config = {
          bucket: values.bucket,
          region: values.region,
          prefix: values.prefix || null,
          accessKeyId: values.accessKeyId,
          period: values.period,
        };

        if (isNewIntegration) {
          await createLogStream({
            logStreamType: "s3Export",
            ...config,
            secretAccessKey: values.secretAccessKey,
          });
          onAddedIntegration?.();
          toast("success", "Created AWS S3 export integration");
        } else {
          // Omitting the secret keeps the one already stored.
          await updateLogStream(logStreamId, {
            logStreamType: "s3Export",
            ...config,
            ...(values.secretAccessKey
              ? { secretAccessKey: values.secretAccessKey }
              : {}),
          });
          toast("success", "Updated AWS S3 export integration");
        }
        onClose();
      } catch (e) {
        helpers.setStatus({
          error: e instanceof Error ? e.message : "Failed to save integration.",
        });
      }
    },
    validationSchema,
  });

  return (
    <form
      onSubmit={formState.handleSubmit}
      className="flex min-h-0 flex-1 flex-col"
    >
      <div className="scrollbar flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-6 pb-4">
        <TextInput
          value={formState.values.bucket}
          onChange={formState.handleChange}
          label="Bucket"
          placeholder="my-convex-mirror"
          id="bucket"
          error={formState.errors.bucket}
          description="The S3 bucket to write the mirror into. Convex does not create the bucket for you."
        />
        <TextInput
          value={formState.values.region}
          onChange={formState.handleChange}
          label="Region"
          placeholder="us-east-1"
          id="region"
          error={formState.errors.region}
          description={
            <>
              The AWS region the bucket lives in.{" "}
              <Link
                href="https://docs.aws.amazon.com/general/latest/gr/s3.html"
                target="_blank"
              >
                See the region list
              </Link>
              .
            </>
          }
        />
        <TextInput
          value={formState.values.prefix}
          onChange={formState.handleChange}
          label="Prefix (optional)"
          placeholder="convex/"
          id="prefix"
          error={formState.errors.prefix}
          description="Key prefix within the bucket. Leave blank to write at the bucket root."
        />
        <TextInput
          value={formState.values.accessKeyId}
          onChange={formState.handleChange}
          label="Access Key ID"
          placeholder="AKIAIOSFODNN7EXAMPLE"
          id="accessKeyId"
          error={formState.errors.accessKeyId}
          description="An AWS access key scoped to writing objects under this bucket and prefix."
        />
        <TextInput
          value={formState.values.secretAccessKey}
          onChange={formState.handleChange}
          type={showSecretAccessKey ? "text" : "password"}
          label={
            isNewIntegration
              ? "Secret Access Key"
              : "Secret Access Key (stored)"
          }
          id="secretAccessKey"
          className="max-w-full"
          placeholder={isNewIntegration ? undefined : "Leave blank to keep"}
          action={() => setShowSecretAccessKey(!showSecretAccessKey)}
          Icon={showSecretAccessKey ? EyeNoneIcon : EyeOpenIcon}
          error={formState.errors.secretAccessKey}
          description={
            isNewIntegration
              ? "The secret for the access key above. It is stored on your deployment and never shown again."
              : "Stored on your deployment and not shown here. Leave blank to keep it, or enter a new secret to replace it."
          }
        />
        <SyncPeriodSelector
          value={formState.values.period}
          onChange={async (period) => {
            await formState.setFieldValue("period", period);
          }}
        />
      </div>
      <div className="flex items-center justify-end gap-2 px-6 py-4">
        {formState.status?.error && (
          <p className="text-sm text-content-errorSecondary" role="alert">
            {formState.status.error}
          </p>
        )}
        <Button
          variant="neutral"
          onClick={onClose}
          disabled={formState.isSubmitting}
        >
          Cancel
        </Button>
        <Button
          variant="primary"
          type="submit"
          aria-label="save"
          disabled={!formState.dirty || formState.isSubmitting}
          loading={formState.isSubmitting}
        >
          Save
        </Button>
      </div>
    </form>
  );
}
