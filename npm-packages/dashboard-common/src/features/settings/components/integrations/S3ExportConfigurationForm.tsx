import { Button } from "@ui/Button";
import { Link } from "@ui/Link";
import { TextInput } from "@ui/TextInput";
import { Combobox, Option } from "@ui/Combobox";
import {
  Disclosure,
  DisclosureButton,
  DisclosurePanel,
} from "@headlessui/react";
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
import { CopyButton } from "@common/elements/CopyButton";
import { SyncPeriodSelector } from "./SyncPeriodSelector";
import {
  exportRoot,
  glueDatabaseName,
  iamPolicy,
  useDeploymentName,
} from "./S3ExportStatus";

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
  const deploymentName = useDeploymentName();

  const storedAccessKeyId = existingConfig?.accessKeyId;
  const validationSchema = useMemo(
    () =>
      Yup.object().shape({
        bucket: bucketSchema,
        region: Yup.string().required("Region is required"),
        // IAM reads `*` and `?` as wildcards, which would widen the generated
        // policy beyond the export's directory.
        prefix: Yup.string().matches(
          /^[^*?]*$/,
          "Prefixes may not contain `*` or `?`",
        ),
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
          toast("success", "Export saved. The first sync is starting.");
        } else {
          // Omitting the secret keeps the one already stored.
          await updateLogStream(logStreamId, {
            logStreamType: "s3Export",
            ...config,
            ...(values.secretAccessKey
              ? { secretAccessKey: values.secretAccessKey }
              : {}),
          });
          toast("success", "Export updated.");
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

  // Formik validates every field on each change; show a field's error only
  // once the user has left it or tried to save.
  const fieldError = (field: keyof typeof formState.values) =>
    formState.touched[field] || formState.submitCount > 0
      ? formState.errors[field]
      : undefined;
  const { bucket, region, prefix } = formState.values;
  const policy = iamPolicy({ bucket, region, prefix, deploymentName });
  // The first invalid destination field, which the policy can't be built
  // without.
  const destinationError = (["bucket", "region", "prefix"] as const)
    .map((field) => {
      try {
        validationSchema.validateSyncAt(field, formState.values);
        return undefined;
      } catch (e) {
        return e instanceof Yup.ValidationError ? e.message : String(e);
      }
    })
    .find((error) => error !== undefined);
  // The policy the user's IAM user has now, as far as the form knows: the one
  // they last copied, or the one for the saved destination.
  const [copiedPolicy, setCopiedPolicy] = useState<string>();
  const appliedPolicy =
    copiedPolicy ??
    (existingConfig &&
      iamPolicy({
        bucket: existingConfig.bucket,
        region: existingConfig.region,
        prefix: existingConfig.prefix ?? "",
        deploymentName,
      }));
  const destinationChanged = !!appliedPolicy && appliedPolicy !== policy;

  return (
    <form
      onSubmit={formState.handleSubmit}
      className="flex min-h-0 flex-1 flex-col"
    >
      <div className="scrollbar flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 pb-4">
        <Section number={1} title="Destination">
          <TextInput
            value={bucket}
            onChange={formState.handleChange}
            label="Bucket name"
            placeholder="my-analytics-bucket"
            id="bucket"
            error={fieldError("bucket")}
            onBlur={formState.handleBlur}
            description="An existing S3 bucket. Convex doesn't create it."
          />
          <div className="flex flex-col gap-1">
            <Combobox
              label="Region"
              labelHidden={false}
              options={AWS_REGIONS}
              selectedOption={region || null}
              setSelectedOption={async (value) => {
                await formState.setFieldValue("region", value ?? "");
                await formState.setFieldTouched("region", true, false);
              }}
              placeholder="Select the bucket's region"
              allowCustomValue
              unknownLabel={(value) => value}
              buttonClasses="w-full bg-inherit"
            />
            {fieldError("region") && (
              <p className="text-xs text-content-errorSecondary" role="alert">
                {fieldError("region")}
              </p>
            )}
          </div>
          <TextInput
            value={prefix}
            onChange={formState.handleChange}
            label="Prefix (optional)"
            placeholder="convex/"
            id="prefix"
            error={fieldError("prefix")}
            onBlur={formState.handleBlur}
          />
          {!destinationError && (
            <p className="text-xs text-content-secondary">
              Convex writes Apache Iceberg tables to{" "}
              <code>
                s3://{bucket}/{exportRoot(prefix, deploymentName)}/tables/
              </code>{" "}
              and registers them in the AWS Glue database{" "}
              <code>{glueDatabaseName(deploymentName)}</code>, which Convex
              creates.
            </p>
          )}
        </Section>
        <Section number={2} title="AWS access">
          <PolicyActions
            policy={policy}
            disabledReason={destinationError}
            destinationChanged={destinationChanged}
            onCopied={() => setCopiedPolicy(policy)}
          >
            {isNewIntegration && (
              <ol className="flex list-[lower-alpha] flex-col gap-1 pl-4 text-xs text-content-secondary">
                <li>
                  In the bucket's AWS account,{" "}
                  <Link
                    href="https://console.aws.amazon.com/iam/home#/users/create"
                    target="_blank"
                  >
                    create an IAM user
                  </Link>
                  .
                </li>
                <li>
                  Open the user, then choose Add permissions, Create inline
                  policy, and JSON. Paste this policy and save it.
                </li>
                <li>
                  Under Security credentials, create an access key for an
                  application running outside AWS. Copy both values: AWS shows
                  the secret only once.
                </li>
              </ol>
            )}
          </PolicyActions>
          <TextInput
            value={formState.values.accessKeyId}
            onChange={formState.handleChange}
            label="Access Key ID"
            placeholder="AKIAIOSFODNN7EXAMPLE"
            id="accessKeyId"
            error={fieldError("accessKeyId")}
            onBlur={formState.handleBlur}
            description={
              isNewIntegration
                ? undefined
                : "To use a new key, create one under the IAM user's Security credentials and enter both values."
            }
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
            iconTooltip={
              showSecretAccessKey
                ? "Hide secret access key"
                : "Show secret access key"
            }
            error={fieldError("secretAccessKey")}
            onBlur={formState.handleBlur}
            description="Permissions can take a minute to apply. Convex retries automatically."
          />
        </Section>
        <Section number={3} title="Frequency">
          <SyncPeriodSelector
            value={formState.values.period}
            onChange={async (period) => {
              await formState.setFieldValue("period", period);
            }}
          />
        </Section>
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

function Section({
  number,
  title,
  children,
}: {
  number: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="flex flex-col gap-3">
      <h5 className="flex items-center gap-2">
        <span className="flex size-5 items-center justify-center rounded-full border text-xs">
          {number}
        </span>
        {title}
      </h5>
      {children}
    </section>
  );
}

function PolicyActions({
  policy,
  disabledReason,
  destinationChanged,
  onCopied,
  children,
}: {
  policy: string;
  disabledReason: string | undefined;
  destinationChanged: boolean;
  onCopied: () => void;
  children: React.ReactNode;
}) {
  return (
    <Disclosure as="div" className="flex flex-col gap-2">
      {children}
      <div className="flex flex-wrap items-center gap-2">
        <CopyButton
          text={policy}
          label="Copy policy"
          size="sm"
          disabled={disabledReason !== undefined}
          onCopied={onCopied}
        />
        <DisclosureButton as={Button} variant="neutral" size="sm">
          {({ open }) => <>{open ? "Hide policy" : "Show policy"}</>}
        </DisclosureButton>
        <p className="text-xs text-content-secondary" aria-live="polite">
          {disabledReason ??
            (destinationChanged &&
              "Destination changed. Copy the updated policy and replace the old one. Your access key stays the same.")}
        </p>
      </div>
      <DisclosurePanel>
        <pre className="scrollbar max-h-64 overflow-auto rounded-sm border bg-background-tertiary p-2 font-mono text-xs">
          {policy}
        </pre>
      </DisclosurePanel>
    </Disclosure>
  );
}

// AWS regions with S3 and Glue. Other region codes can be typed in.
const AWS_REGIONS: Option<string>[] = [
  ["us-east-1", "US East (N. Virginia)"],
  ["us-east-2", "US East (Ohio)"],
  ["us-west-1", "US West (N. California)"],
  ["us-west-2", "US West (Oregon)"],
  ["af-south-1", "Africa (Cape Town)"],
  ["ap-east-1", "Asia Pacific (Hong Kong)"],
  ["ap-south-1", "Asia Pacific (Mumbai)"],
  ["ap-south-2", "Asia Pacific (Hyderabad)"],
  ["ap-southeast-1", "Asia Pacific (Singapore)"],
  ["ap-southeast-2", "Asia Pacific (Sydney)"],
  ["ap-southeast-3", "Asia Pacific (Jakarta)"],
  ["ap-southeast-4", "Asia Pacific (Melbourne)"],
  ["ap-northeast-1", "Asia Pacific (Tokyo)"],
  ["ap-northeast-2", "Asia Pacific (Seoul)"],
  ["ap-northeast-3", "Asia Pacific (Osaka)"],
  ["ca-central-1", "Canada (Central)"],
  ["ca-west-1", "Canada West (Calgary)"],
  ["eu-central-1", "Europe (Frankfurt)"],
  ["eu-central-2", "Europe (Zurich)"],
  ["eu-west-1", "Europe (Ireland)"],
  ["eu-west-2", "Europe (London)"],
  ["eu-west-3", "Europe (Paris)"],
  ["eu-south-1", "Europe (Milan)"],
  ["eu-south-2", "Europe (Spain)"],
  ["eu-north-1", "Europe (Stockholm)"],
  ["il-central-1", "Israel (Tel Aviv)"],
  ["me-south-1", "Middle East (Bahrain)"],
  ["me-central-1", "Middle East (UAE)"],
  ["sa-east-1", "South America (São Paulo)"],
].map(([value, name]) => ({ value, label: `${name} ${value}` }));
