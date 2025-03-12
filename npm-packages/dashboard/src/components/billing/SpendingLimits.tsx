import * as Yup from "yup";
import React, { useCallback, useEffect, useId } from "react";
import { TextInput } from "@common/elements/TextInput";
import { Form, Formik, getIn, useFormikContext } from "formik";
import { Button } from "dashboard-common/elements/Button";
import { Team } from "generatedApi";
import { useSetSpendingLimit } from "api/billing";
import { Loading } from "dashboard-common/elements/Loading";
import * as Sentry from "@sentry/nextjs";
import { Checkbox } from "dashboard-common/elements/Checkbox";
import { cn } from "dashboard-common/lib/cn";
import {
  ExclamationTriangleIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { Tooltip } from "dashboard-common/elements/Tooltip";
import Link from "next/link";
import { formatUsd } from "dashboard-common/lib/utils";

export type SpendingLimitsValue = {
  // null = disabled (= checkbox unchecked)
  // undefined = enabled but no value set

  spendingLimitWarningThresholdUsd: null | undefined | number;
  spendingLimitDisableThresholdUsd: null | undefined | number;
};

export function spendingLimitsSchema({
  currentSpendingUsd,
}: {
  currentSpendingUsd: number | undefined;
}) {
  if (currentSpendingUsd && currentSpendingUsd < 0) {
    Sentry.captureMessage("Negative spending");
  }

  const baseSchema = Yup.mixed()
    .test(
      "is-spending-value",
      "Please enter a positive number.",
      (value) => value === null || (typeof value === "number" && value >= 0),
    )
    .test(
      "is-integer-or-null",
      "Please enter an integer amount.",
      (value) => value === null || Number.isInteger(value),
    );

  const disableSchema = baseSchema.test(
    "is-greater-than-current-spending",
    `The spend limit must be greater than the spending in the current billing cycle (${formatUsd(currentSpendingUsd ?? -1)}).`,
    (value) =>
      currentSpendingUsd === undefined ||
      value === null ||
      currentSpendingUsd <= value,
  );

  return Yup.object().shape({
    spendingLimitDisableThresholdUsd: disableSchema,
    spendingLimitWarningThresholdUsd: baseSchema.test(
      "is-less-than-spend-limit",
      "The warning threshold must be less than the spend limit.",
      function isLessThanSpendLimitValidator(warningThreshold) {
        const { parent } = this;
        const disableThreshold = parent.spendingLimitDisableThresholdUsd;

        // If the disable threshold is 0, this field should always be valid
        // (it will be hidden in the UI and set to null)
        if (disableThreshold === 0) {
          return true;
        }

        return (
          typeof warningThreshold !== "number" ||
          typeof disableThreshold !== "number" ||
          warningThreshold < disableThreshold
        );
      },
    ),
  });
}

export function useSubmitSpendingLimits(team: Team) {
  const setSpendingLimit = useSetSpendingLimit(team.id);

  return useCallback(
    async (v: SpendingLimitsValue) => {
      await setSpendingLimit({
        warningThresholdCents:
          typeof v.spendingLimitWarningThresholdUsd === "number"
            ? v.spendingLimitWarningThresholdUsd * 100
            : v.spendingLimitWarningThresholdUsd,
        disableThresholdCents:
          typeof v.spendingLimitDisableThresholdUsd === "number"
            ? v.spendingLimitDisableThresholdUsd * 100
            : v.spendingLimitDisableThresholdUsd,
      });
    },
    [setSpendingLimit],
  );
}

export function SpendingLimitsForm({
  defaultValue,
  onSubmit,
  onCancel,
  currentSpendingUsd,
}: {
  defaultValue: SpendingLimitsValue | undefined;
  onSubmit: (values: SpendingLimitsValue) => Promise<void>;
  onCancel: () => void;
  currentSpendingUsd: number | undefined;
}) {
  const isLoading = defaultValue === undefined;

  return (
    <Formik
      enableReinitialize
      initialValues={
        isLoading
          ? {
              spendingLimitWarningThresholdUsd: null,
              spendingLimitDisableThresholdUsd: null,
            }
          : defaultValue
      }
      validationSchema={spendingLimitsSchema({ currentSpendingUsd })}
      onSubmit={async (e) => {
        await onSubmit(e);
      }}
    >
      {({ isSubmitting, isValid }) => (
        <Form
          className="flex flex-col items-start gap-4"
          placeholder={undefined}
          onPointerEnterCapture={undefined}
          onPointerLeaveCapture={undefined}
        >
          {isLoading ? (
            <Loading className="h-[176px] w-full max-w-64" fullHeight={false} />
          ) : (
            <SpendingLimits />
          )}

          <div className="flex gap-2">
            <Button
              type="submit"
              disabled={isLoading || isSubmitting || !isValid}
            >
              {isSubmitting
                ? "Saving Spending Limits…"
                : "Save Spending Limits"}
            </Button>
            <Button variant="neutral" onClick={onCancel}>
              Cancel
            </Button>
          </div>
        </Form>
      )}
    </Formik>
  );
}

/** To use within a Formik form with a state that is a superset of `SpendingLimitsValue` */
export function SpendingLimits() {
  const { values, setFieldValue, touched } =
    useFormikContext<SpendingLimitsValue>();
  const { spendingLimitWarningThresholdUsd, spendingLimitDisableThresholdUsd } =
    values;

  useEffect(() => {
    if (spendingLimitDisableThresholdUsd === 0) {
      setFieldValue("spendingLimitWarningThresholdUsd", null);
    }
  }, [
    spendingLimitDisableThresholdUsd,
    setFieldValue,
    touched.spendingLimitDisableThresholdUsd,
  ]);

  return (
    <div className="flex w-full flex-col gap-4">
      {spendingLimitDisableThresholdUsd !== 0 && (
        <SpendLimitInput
          formKey="spendingLimitWarningThresholdUsd"
          label="Warn when spending exceeds"
          accessibleInputLabel="Warning Threshold"
          disabled={spendingLimitDisableThresholdUsd === 0}
          description={
            spendingLimitWarningThresholdUsd !== null && (
              <>
                If your <UsageDefinition /> exceeds this amount,
                <br />
                admins in your team will be notified by email.
              </>
            )
          }
        />
      )}
      <SpendLimitInput
        formKey="spendingLimitDisableThresholdUsd"
        label="Limit usage spending to"
        accessibleInputLabel="Disable Threshold"
        description={
          spendingLimitDisableThresholdUsd !== null && (
            <span className="mt-0.5 flex gap-1.5 text-content-warning">
              <ExclamationTriangleIcon />
              <span className="block flex-1">
                If your <UsageDefinition /> exceeds{" "}
                {spendingLimitDisableThresholdUsd === 0
                  ? "the built-in limits of your plan, "
                  : "this amount, "}
                <strong className="font-semibold">
                  all of your team’s projects will be disabled
                </strong>{" "}
                until you increase the spend limit.
              </span>
            </span>
          )
        }
      />
    </div>
  );
}

function UsageDefinition() {
  return (
    <Tooltip
      tip={
        <>
          Resources used beyond the{" "}
          <Link
            className="text-content-link hover:underline"
            href="https://www.convex.dev/pricing"
          >
            built-in resources of your plan
          </Link>
          . Seat fees are not counted in your spending limits.
        </>
      }
      side="right"
    >
      <div className="flex gap-0.5">
        <span className="underline decoration-dotted">usage</span>
        <QuestionMarkCircledIcon />
      </div>
    </Tooltip>
  );
}

function SpendLimitInput({
  formKey,
  label,
  accessibleInputLabel,
  description,
  disabled = false,
}: {
  formKey:
    | "spendingLimitWarningThresholdUsd"
    | "spendingLimitDisableThresholdUsd";
  label: string;
  accessibleInputLabel: string;
  description?: React.ReactNode;
  disabled?: boolean;
}) {
  const formState = useFormikContext<SpendingLimitsValue>();
  const error = formState.errors[formKey];
  const value = formState.values[formKey];

  const checkboxId = useId();
  const inputId = useId();

  const inputDisabled = value === null || disabled;

  return (
    <div className="flex max-w-64 flex-col gap-1">
      <label
        className="flex items-center gap-2 text-sm text-content-primary"
        htmlFor={checkboxId}
      >
        <Checkbox
          id={checkboxId}
          checked={value !== null}
          onChange={() => {
            formState.setFieldValue(formKey, value === null ? undefined : null);
          }}
          disabled={disabled}
        />
        {label}
      </label>

      <TextInput
        id={inputId}
        type="number"
        {...formState.getFieldProps(formKey)}
        value={value ?? ""}
        label={accessibleInputLabel}
        labelHidden
        description={description}
        min={0}
        step={1}
        error={getIn(formState.touched, formKey) && error}
        leftAddon={
          <div
            className={cn(
              "w-4 text-center text-sm",
              inputDisabled ? "text-content-tertiary" : "text-content-primary",
            )}
          >
            $
          </div>
        }
        rightAddon={
          <div className="text-sm text-content-secondary">/ month</div>
        }
        className="pr-16"
        disabled={inputDisabled}
      />
    </div>
  );
}
