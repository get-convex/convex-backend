import { useState, useCallback, useId, useMemo } from "react";
import {
  useCurrentDeployment,
  useModifyDeploymentSettings,
} from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import { useHasProjectAdminPermissions } from "api/roles";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { Checkbox } from "@ui/Checkbox";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TextInput } from "@ui/TextInput";
import {
  Pencil1Icon,
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import { CopyButton } from "@common/elements/CopyButton";
import { LiveTimestampDistance } from "@common/elements/TimestampDistance";
import { cn } from "@ui/cn";
import { useFormik } from "formik";
import * as Yup from "yup";
import * as Sentry from "@sentry/nextjs";
import type { DeploymentType } from "@convex-dev/platform/managementApi";

const referenceValidationSchema = Yup.object().shape({
  reference: Yup.string()
    .required("Reference is required")
    .min(3, "Reference must be at least 3 characters")
    .max(100, "Reference must be at most 100 characters")
    .matches(
      /^[a-z0-9/-]+$/,
      "Reference can only contain lowercase letters, numbers, hyphens, and slashes",
    )
    .test(
      "not-deployment-name-format",
      "Reference cannot be in the format abc-xyz-123, as it is reserved for deployment names",
      (value) => {
        if (!value) return true;
        return !/^[a-z]+-[a-z]+-\d+$/.test(value);
      },
    )
    .test(
      "not-local-prefix",
      "Reference cannot start with 'local-'",
      (value) => {
        if (!value) return true;
        return !value.startsWith("local-");
      },
    )
    .test(
      "not-reserved",
      // eslint-disable-next-line no-template-curly-in-string -- Yup error template
      '"${value}" is a reserved name and cannot be used as a reference.',
      (value) => {
        if (!value) return true;
        const reserved = [
          "prod",
          "dev",
          "cloud",
          "local",
          "default",
          "name",
          "new",
          "existing",
          "deployment",
          "preview",
        ];
        return !reserved.includes(value);
      },
    ),
});

type TriStateValue = boolean | null;

function getDefaultSendLogsToClient(deploymentType: DeploymentType): boolean {
  return deploymentType === "dev" || deploymentType === "preview";
}

function getDefaultDashboardEditConfirmation(
  deploymentType: DeploymentType,
): boolean {
  return deploymentType === "prod";
}

function triStateLabel(defaultValue: boolean): string {
  return defaultValue ? "(enabled)" : "(disabled)";
}

function isSecurityWarningDeployment(deploymentType: DeploymentType): boolean {
  return deploymentType === "prod" || deploymentType === "custom";
}

function triStateDisplayValue(value: TriStateValue): string {
  if (value === null) return "Default";
  return value ? "Enabled" : "Disabled";
}

const THIRTY_MINUTES_MS = 30 * 60 * 1000;

function toDateTimeLocalValue(d: Date): string {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  const hours = String(d.getHours()).padStart(2, "0");
  const minutes = String(d.getMinutes()).padStart(2, "0");
  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

function TriStateRadioGroup({
  name,
  value,
  onChange,
  defaultForType,
  defaultTooltip,
  disabled,
}: {
  name: string;
  value: TriStateValue;
  onChange: (value: TriStateValue) => void;
  defaultForType: boolean;
  defaultTooltip?: string;
  disabled: boolean;
}) {
  const options: { optionValue: TriStateValue; optionLabel: string }[] = [
    {
      optionValue: null,
      optionLabel: `Default ${triStateLabel(defaultForType)}`,
    },
    { optionValue: true, optionLabel: "Enabled" },
    { optionValue: false, optionLabel: "Disabled" },
  ];

  return (
    <div
      role="radiogroup"
      aria-label={name}
      className={cn(
        "flex flex-col gap-1.5",
        disabled && "cursor-not-allowed opacity-50",
      )}
    >
      {options.map(({ optionValue, optionLabel }, index) => {
        const isSelected = value === optionValue;
        return (
          <div
            key={String(optionValue)}
            role="radio"
            aria-checked={isSelected}
            tabIndex={disabled ? -1 : 0}
            onClick={() => {
              if (!disabled) onChange(optionValue);
            }}
            onKeyDown={(e) => {
              if (disabled) return;
              if (e.key === " " || e.key === "Enter") {
                e.preventDefault();
                onChange(optionValue);
              }
              if (e.key === "ArrowDown" || e.key === "ArrowRight") {
                e.preventDefault();
                const next = options[(index + 1) % options.length];
                onChange(next.optionValue);
                (
                  e.currentTarget.parentElement?.children[
                    (index + 1) % options.length
                  ] as HTMLElement
                )?.focus();
              }
              if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
                e.preventDefault();
                const prev =
                  options[(index - 1 + options.length) % options.length];
                onChange(prev.optionValue);
                (
                  e.currentTarget.parentElement?.children[
                    (index - 1 + options.length) % options.length
                  ] as HTMLElement
                )?.focus();
              }
            }}
            className={cn(
              "flex items-center gap-2 text-sm",
              disabled ? "cursor-not-allowed" : "cursor-pointer",
              "rounded focus:outline-none focus-visible:ring-2 focus-visible:ring-util-accent",
            )}
          >
            <div
              className={cn(
                "flex size-4 shrink-0 items-center justify-center rounded-full border",
                isSelected
                  ? "border-util-accent bg-util-accent"
                  : "border-border-transparent bg-background-secondary",
              )}
            >
              {isSelected && <div className="size-1.5 rounded-full bg-white" />}
            </div>
            {optionLabel}
            {optionValue === null && defaultTooltip && (
              <Tooltip tip={defaultTooltip} side="right">
                <InfoCircledIcon className="size-3.5 text-content-secondary" />
              </Tooltip>
            )}
          </div>
        );
      })}
    </div>
  );
}

type SaveFn = (args: {
  sendLogsToClient?: boolean | null;
  dashboardEditConfirmation?: boolean | null;
  expiresAt?: number | null;
  reference?: string;
}) => Promise<unknown>;

export function DeploymentAdvancedSettings() {
  const deployment = useCurrentDeployment();
  const project = useCurrentProject();
  const team = useCurrentTeam();
  const entitlements = useTeamEntitlements(team?.id);
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);
  const modifySettings = useModifyDeploymentSettings({
    deploymentName: deployment?.name,
    projectId: project?.id,
  });

  if (deployment === undefined) return null;
  if (deployment.kind === "local") return null;

  const disabled = !hasAdminPermissions;
  const deploymentType = deployment.deploymentType;

  return (
    <>
      <DeploymentReferenceSheet
        reference={deployment.reference}
        disabled={disabled}
        onSave={modifySettings}
      />
      <TriStateSettingSheet
        title="Send Logs to Client"
        description="Whether function logs and errors are sent to the calling client."
        value={deployment.sendLogsToClient ?? null}
        deploymentType={deploymentType}
        defaultForType={getDefaultSendLogsToClient(deploymentType)}
        defaultTooltip={
          getDefaultSendLogsToClient(deploymentType)
            ? "Logs are enabled by default for dev and preview deployments to aid debugging."
            : "Logs are disabled by default for production deployments to avoid exposing sensitive information."
        }
        getWarning={(value, defaultValue) => {
          if (!isSecurityWarningDeployment(deploymentType) || value === null) {
            return null;
          }
          if (value === true && !defaultValue) {
            return "Sending logs to the client may expose sensitive information in production.";
          }
          if (value === false && defaultValue) {
            return "Disabling client logs will make debugging more difficult.";
          }
          return null;
        }}
        fieldName="sendLogsToClient"
        disabled={disabled}
        onSave={modifySettings}
      />
      <TriStateSettingSheet
        title="Dashboard Edit Confirmation"
        description="Whether the dashboard requires confirmation before allowing edits."
        value={deployment.dashboardEditConfirmation ?? null}
        deploymentType={deploymentType}
        defaultForType={getDefaultDashboardEditConfirmation(deploymentType)}
        defaultTooltip={
          getDefaultDashboardEditConfirmation(deploymentType)
            ? "Edit confirmation is enabled by default for production deployments to prevent accidental changes."
            : "Edit confirmation is disabled by default for dev and preview deployments for faster iteration."
        }
        getWarning={(value, defaultValue) => {
          if (!isSecurityWarningDeployment(deploymentType) || value === null) {
            return null;
          }
          if (value === false && defaultValue) {
            return "Disabling edit confirmation may lead to accidental changes in production.";
          }
          return null;
        }}
        fieldName="dashboardEditConfirmation"
        disabled={disabled}
        onSave={modifySettings}
      />
      <DeploymentExpirySheet
        expiresAt={deployment.expiresAt ?? null}
        deploymentType={deploymentType}
        previewRetentionDays={entitlements?.previewDeploymentRetentionDays}
        disabled={disabled}
        onSave={modifySettings}
      />
    </>
  );
}

function DeploymentReferenceSheet({
  reference,
  disabled,
  onSave,
}: {
  reference: string;
  disabled: boolean;
  onSave: SaveFn;
}) {
  const referenceFieldId = useId();
  const [isEditing, setIsEditing] = useState(false);

  const form = useFormik({
    initialValues: { reference },
    validationSchema: referenceValidationSchema,
    onSubmit: async (values) => {
      if (values.reference === undefined) {
        Sentry.captureMessage(
          "Unexpectedly submitting DeploymentReferenceInner with an undefined value",
          "error",
        );
        return;
      }
      await onSave({ reference: values.reference });
      setIsEditing(false);
    },
    enableReinitialize: true,
  });

  const handleCancel = useCallback(() => {
    form.resetForm();
    setIsEditing(false);
  }, [form]);

  return (
    <Sheet>
      <h4 className="mb-2">Deployment Reference</h4>
      <p className="mb-4 text-xs text-content-secondary">
        You can use the reference to target this deployment from the CLI (e.g.{" "}
        <code>--deployment&nbsp;{reference ?? "<reference>"}</code>).
      </p>
      <div className="flex flex-wrap items-start gap-x-2 gap-y-4 sm:flex-nowrap">
        {!isEditing ? (
          <>
            <TextInput
              id={referenceFieldId}
              label="Reference"
              labelHidden
              value={reference}
              disabled
            />
            <CopyButton
              text={reference ?? ""}
              disabled={reference === undefined}
              size="sm"
            />
            <Button
              variant="neutral"
              onClick={() => setIsEditing(true)}
              disabled={disabled}
              icon={<Pencil1Icon />}
              aria-label="Edit deployment reference"
            >
              Edit
            </Button>
          </>
        ) : (
          <form onSubmit={form.handleSubmit} className="contents">
            <TextInput
              id={referenceFieldId}
              label="Reference"
              labelHidden
              error={
                (form.touched.reference && form.errors.reference) || undefined
              }
              disabled={form.isSubmitting}
              {...form.getFieldProps("reference")}
            />
            <Button
              type="button"
              variant="neutral"
              onClick={handleCancel}
              disabled={form.isSubmitting}
            >
              Undo Edit
            </Button>
            <Button
              type="submit"
              variant="primary"
              disabled={form.isSubmitting || !form.isValid}
              loading={form.isSubmitting}
            >
              Save
            </Button>
          </form>
        )}
      </div>
    </Sheet>
  );
}

function TriStateSettingSheet({
  title,
  description,
  value: initialValue,
  deploymentType,
  defaultForType,
  defaultTooltip,
  getWarning,
  fieldName,
  disabled,
  onSave,
}: {
  title: string;
  description: string;
  value: TriStateValue;
  deploymentType: DeploymentType;
  defaultForType: boolean;
  defaultTooltip?: string;
  getWarning: (value: TriStateValue, defaultValue: boolean) => string | null;
  fieldName: "sendLogsToClient" | "dashboardEditConfirmation";
  disabled: boolean;
  onSave: SaveFn;
}) {
  const [value, setValue] = useState<TriStateValue>(initialValue);
  const [isSaving, setIsSaving] = useState(false);
  const [showConfirmation, setShowConfirmation] = useState(false);

  const isDirty = value !== initialValue;

  const warningText = getWarning(value, defaultForType);

  const executeSave = useCallback(async () => {
    setIsSaving(true);
    try {
      await onSave({ [fieldName]: value });
    } finally {
      setIsSaving(false);
    }
  }, [onSave, fieldName, value]);

  const handleSave = useCallback(() => {
    setShowConfirmation(true);
  }, []);

  return (
    <Sheet>
      <h4 className="mb-2">{title}</h4>
      <p className="mb-4 text-xs text-content-secondary">{description}</p>
      <div className="flex flex-col gap-3">
        <TriStateRadioGroup
          name={fieldName}
          value={value}
          onChange={setValue}
          defaultForType={defaultForType}
          defaultTooltip={defaultTooltip}
          disabled={disabled || isSaving}
        />
        {warningText && (
          <div className="flex w-fit items-center gap-2 rounded-lg border bg-background-warning px-3 py-2 text-sm text-content-warning">
            <ExclamationTriangleIcon className="size-4 shrink-0" />
            <span>{warningText}</span>
          </div>
        )}
        <div className="flex justify-end">
          <Button
            variant="primary"
            disabled={!isDirty || isSaving || disabled}
            loading={isSaving}
            onClick={handleSave}
          >
            Save
          </Button>
        </div>
      </div>

      {showConfirmation && (
        <ConfirmationDialog
          onClose={() => setShowConfirmation(false)}
          onConfirm={executeSave}
          dialogTitle={`Confirm ${title} Change`}
          dialogBody={
            <div className="flex flex-col gap-3 text-sm">
              <p>
                You are modifying settings on a {deploymentType} deployment.
              </p>
              <div className="flex flex-col gap-2 rounded-md border bg-background-tertiary p-3">
                <div className="flex flex-col gap-0.5">
                  <span className="font-medium">{title}</span>
                  <span className="text-content-secondary">
                    {triStateDisplayValue(initialValue)} →{" "}
                    {triStateDisplayValue(value)}
                  </span>
                </div>
              </div>
            </div>
          }
          confirmText="Save Changes"
          variant="danger"
        />
      )}
    </Sheet>
  );
}

function DeploymentExpirySheet({
  expiresAt: initialExpiresAt,
  deploymentType,
  previewRetentionDays,
  disabled,
  onSave,
}: {
  expiresAt: number | null;
  deploymentType: DeploymentType;
  previewRetentionDays: number | undefined;
  disabled: boolean;
  onSave: SaveFn;
}) {
  const [hasExpiry, setHasExpiry] = useState(initialExpiresAt !== null);
  const defaultExpiryMs =
    previewRetentionDays !== undefined
      ? previewRetentionDays * 24 * 60 * 60 * 1000
      : THIRTY_MINUTES_MS;
  const [expiryDate, setExpiryDate] = useState<Date>(
    initialExpiresAt !== null
      ? new Date(initialExpiresAt)
      : new Date(Date.now() + defaultExpiryMs),
  );
  const [isSaving, setIsSaving] = useState(false);
  const [showConfirmation, setShowConfirmation] = useState(false);

  const minExpiryDate = useMemo(
    () => new Date(Date.now() + THIRTY_MINUTES_MS),
    [],
  );
  const maxExpiryDate = useMemo(() => {
    if (previewRetentionDays === undefined) return undefined;
    return new Date(Date.now() + previewRetentionDays * 24 * 60 * 60 * 1000);
  }, [previewRetentionDays]);

  const isDirty =
    hasExpiry !== (initialExpiresAt !== null) ||
    (hasExpiry && expiryDate.getTime() !== initialExpiresAt);

  const isExpiryValid =
    !hasExpiry ||
    (expiryDate.getTime() >= minExpiryDate.getTime() &&
      (maxExpiryDate === undefined ||
        expiryDate.getTime() <= maxExpiryDate.getTime()));

  const executeSave = useCallback(async () => {
    setIsSaving(true);
    try {
      await onSave({ expiresAt: hasExpiry ? expiryDate.getTime() : null });
    } finally {
      setIsSaving(false);
    }
  }, [onSave, hasExpiry, expiryDate]);

  const handleSave = useCallback(() => {
    setShowConfirmation(true);
  }, []);

  return (
    <Sheet>
      <h4 className="mb-2">Deployment Expiry</h4>
      <p className="mb-4 text-xs text-content-secondary">
        Set a time at which this deployment will be automatically deleted.
      </p>
      <div className="flex flex-col gap-3">
        <label
          className={cn(
            "flex items-center gap-2 text-sm",
            (disabled || isSaving) && "cursor-not-allowed opacity-50",
          )}
        >
          <Checkbox
            checked={hasExpiry}
            onChange={() => setHasExpiry(!hasExpiry)}
            disabled={disabled || isSaving}
          />
          <span>This deployment will expire at</span>
          {hasExpiry ? (
            <input
              type="datetime-local"
              value={toDateTimeLocalValue(expiryDate)}
              min={toDateTimeLocalValue(minExpiryDate)}
              max={
                maxExpiryDate ? toDateTimeLocalValue(maxExpiryDate) : undefined
              }
              onChange={(e) => {
                if (e.target.value) {
                  setExpiryDate(new Date(e.target.value));
                }
              }}
              disabled={disabled || isSaving}
              className={cn(
                "w-fit rounded-md border bg-background-secondary px-3 py-1.5 text-sm text-content-primary",
                "focus:border-border-selected focus:outline-none",
                "disabled:cursor-not-allowed disabled:opacity-50",
              )}
            />
          ) : (
            <span
              className={cn(
                "w-fit rounded-md border bg-background-secondary px-3 py-1.5 text-sm text-content-secondary",
                "cursor-not-allowed opacity-50",
              )}
            >
              Never
            </span>
          )}
          {hasExpiry && <LiveTimestampDistance date={expiryDate} />}
        </label>
        {hasExpiry && !isExpiryValid && (
          <div className="flex w-fit items-center gap-2 rounded-lg border bg-background-error px-3 py-2 text-sm text-content-error">
            <ExclamationTriangleIcon className="size-4 shrink-0" />
            <span>
              The expiry time must be between 30 minutes and{" "}
              {previewRetentionDays} days from now.
            </span>
          </div>
        )}
        <div className="flex justify-end">
          <Button
            variant="primary"
            disabled={!isDirty || isSaving || disabled || !isExpiryValid}
            loading={isSaving}
            onClick={handleSave}
          >
            Save
          </Button>
        </div>
      </div>

      {showConfirmation && (
        <ConfirmationDialog
          onClose={() => setShowConfirmation(false)}
          onConfirm={executeSave}
          dialogTitle="Confirm Deployment Expiry Change"
          dialogBody={
            <div className="flex flex-col gap-3 text-sm">
              <p>
                You are modifying settings on a {deploymentType} deployment.
              </p>
              <div className="flex flex-col gap-2 rounded-md border bg-background-tertiary p-3">
                <div className="flex flex-col gap-0.5">
                  <span className="font-medium">Deployment Expiry</span>
                  <span className="text-content-secondary">
                    {initialExpiresAt !== null
                      ? new Date(initialExpiresAt).toLocaleString(undefined, {
                          timeZoneName: "short",
                        })
                      : "None"}{" "}
                    →{" "}
                    {hasExpiry
                      ? expiryDate.toLocaleString(undefined, {
                          timeZoneName: "short",
                        })
                      : "None"}
                  </span>
                </div>
              </div>
            </div>
          }
          confirmText="Save Changes"
          variant="danger"
        />
      )}
    </Sheet>
  );
}
