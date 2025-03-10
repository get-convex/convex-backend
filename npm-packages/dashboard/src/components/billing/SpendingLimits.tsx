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

export type SpendingLimitsValue = {
  spendingLimitEnabled: boolean;
  spendingLimitDisableThresholdUsd: number | null;
  spendingLimitWarningThresholdUsd: number | null;
};

export function spendingLimitsSchema(currentSpendingUsd: number | undefined) {
  if (currentSpendingUsd && currentSpendingUsd < 0) {
    Sentry.captureMessage("Negative spending");
  }

  return Yup.object().shape({
    spendingLimitEnabled: Yup.boolean().required(),
    spendingLimitDisableThresholdUsd: Yup.mixed().when("spendingLimitEnabled", {
      is: (spendingLimitEnabled: boolean) => spendingLimitEnabled,
      then: Yup.number()
        .typeError("Please enter a number.")
        .required("Please enter a number.")
        .min(
          currentSpendingUsd ?? 0,
          currentSpendingUsd
            ? `The spend limit must be greater than the spending in the current billing cycle (${formatUsd(currentSpendingUsd)}).`
            : "Please enter a positive number.",
        )
        .integer("Please enter an integer amount."),
      otherwise: Yup.mixed().nullable(),
    }),
    spendingLimitWarningThresholdUsd: Yup.mixed().when(
      "spendingLimitDisableThresholdUsd",
      {
        is: (spendingLimitDisableThresholdUsd: number | null) =>
          spendingLimitDisableThresholdUsd !== null &&
          spendingLimitDisableThresholdUsd > 0,
        then: Yup.number()
          .typeError("Please enter a number.")
          .min(0, "Please enter a positive number.")
          .required("Please enter a warning threshold.")
          .lessThan(
            Yup.ref("spendingLimitDisableThresholdUsd"),
            "The warning threshold must be lower than the spend limit.",
          )
          .integer("Please enter an integer amount."),
        otherwise: Yup.mixed().nullable(),
      },
    ),
  });
}

export function useSubmitSpendingLimits(team: Team) {
  const setSpendingLimit = useSetSpendingLimit(team.id);

  return useCallback(
    async (v: SpendingLimitsValue) => {
      await setSpendingLimit({
        disableThresholdCents:
          v.spendingLimitDisableThresholdUsd === null
            ? null
            : v.spendingLimitDisableThresholdUsd * 100,
        warningThresholdCents:
          v.spendingLimitWarningThresholdUsd === null
            ? null
            : v.spendingLimitWarningThresholdUsd * 100,
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
              spendingLimitEnabled: false,
              spendingLimitDisableThresholdUsd: null,
              spendingLimitWarningThresholdUsd: null,
            }
          : defaultValue
      }
      validationSchema={spendingLimitsSchema(currentSpendingUsd)}
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
            <Loading className="h-[214px] w-full" fullHeight={false} />
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
  const {
    spendingLimitEnabled,
    spendingLimitDisableThresholdUsd,
    spendingLimitWarningThresholdUsd,
  } = values;

  useEffect(() => {
    if (spendingLimitDisableThresholdUsd === 0) {
      setFieldValue("spendingLimitWarningThresholdUsd", null);
    }

    if (!spendingLimitEnabled) {
      setFieldValue("spendingLimitDisableThresholdUsd", null);
    }

    if (
      spendingLimitDisableThresholdUsd !== null &&
      touched.spendingLimitDisableThresholdUsd &&
      !touched.spendingLimitWarningThresholdUsd &&
      spendingLimitWarningThresholdUsd === null
    ) {
      setFieldValue(
        "spendingLimitWarningThresholdUsd",
        Math.floor(spendingLimitDisableThresholdUsd * 0.8),
      );
    }

    // Ignoring updates to `spendingLimitWarningThresholdUsd`
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    spendingLimitDisableThresholdUsd,
    setFieldValue,
    spendingLimitEnabled,
    touched.spendingLimitDisableThresholdUsd,
    touched.spendingLimitWarningThresholdUsd,
  ]);

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="flex flex-col gap-1">
        <SpendLimitToggle />
        <SpendLimitInput
          formKey="spendingLimitDisableThresholdUsd"
          label="Spend Limit"
          labelHidden
          disabled={!spendingLimitEnabled}
          description={
            spendingLimitEnabled && (
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
      {spendingLimitDisableThresholdUsd !== 0 && (
        <SpendLimitInput
          formKey="spendingLimitWarningThresholdUsd"
          label="Warn when spending exceeds"
          disabled={spendingLimitDisableThresholdUsd === 0}
          description={
            <>
              If your <UsageDefinition /> exceeds this amount,
              <br />
              admins in your team will be notified by email.
            </>
          }
        />
      )}
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

function SpendLimitToggle() {
  const { values, setFieldValue } = useFormikContext<SpendingLimitsValue>();
  const id = useId();

  return (
    <label
      className="flex items-center gap-2 text-sm text-content-primary"
      htmlFor={id}
    >
      <Checkbox
        id={id}
        checked={values.spendingLimitEnabled}
        onChange={() => {
          setFieldValue("spendingLimitEnabled", !values.spendingLimitEnabled);
          setFieldValue("spendingLimitDisableThresholdUsd", null);
        }}
      />
      Limit usage spending to
    </label>
  );
}

function SpendLimitInput({
  formKey,
  label,
  labelHidden = false,
  description,
  disabled,
}: {
  formKey:
    | "spendingLimitDisableThresholdUsd"
    | "spendingLimitWarningThresholdUsd";
  label: string;
  labelHidden?: boolean;
  description?: React.ReactNode;
  disabled?: boolean;
}) {
  const formState = useFormikContext<SpendingLimitsValue>();
  const error = formState.errors[formKey];
  const value = formState.values[formKey];

  const id = useId();

  return (
    <div className="max-w-64">
      <TextInput
        id={id}
        type="number"
        {...formState.getFieldProps(formKey)}
        value={value ?? ""}
        label={label}
        labelHidden={labelHidden}
        description={description}
        min={0}
        step={1}
        error={getIn(formState.touched, formKey) && error}
        leftAddon={
          <div
            className={cn(
              "w-4 text-center text-sm",
              disabled ? "text-content-tertiary" : "text-content-primary",
            )}
          >
            $
          </div>
        }
        rightAddon={
          <div className="text-sm text-content-secondary">/ month</div>
        }
        className="pr-16"
        disabled={disabled}
      />
    </div>
  );
}

export function formatUsd(usd: number) {
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(usd);
}
