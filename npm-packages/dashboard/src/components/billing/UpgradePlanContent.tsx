import { Button } from "dashboard-common/elements/Button";
import { Spinner } from "dashboard-common/elements/Spinner";
import { TextInput } from "dashboard-common/elements/TextInput";
import React, { useCallback, useEffect, useState } from "react";
import { useDebounce } from "react-use";
import { Elements } from "@stripe/react-stripe-js";
import { useGetCoupon, useCreateSubscription } from "api/billing";
import { FormikProvider, useFormik, useFormikContext } from "formik";
import * as Yup from "yup";
import { Address, PlanResponse, Team } from "generatedApi";
import { PaymentDetailsForm } from "./PaymentDetailsForm";
import { BillingAddressInputs } from "./BillingAddressInputs";
import { useStripePaymentSetup } from "../../hooks/useStripe";
import { BillingContactInputs } from "./BillingContactInputs";
import { SpendingLimits, spendingLimitsSchema } from "./SpendingLimits";
import { UpgradeFormState } from "./upgradeFormState";
import { useLaunchDarkly } from "../../hooks/useLaunchDarkly";

export const debounceDurationMs = 200;

export type UpgradePlanContentProps = {
  plan: PlanResponse;
  couponDurationInMonths?: number;
  numMembers: number;
  teamMemberDiscountPct?: number;
  requiresPaymentMethod?: boolean;
  isLoadingPromo?: boolean;
  promoCodeError?: string;
  setPaymentMethod: (paymentMethod?: string) => void;
  billingAddressInputs: React.ReactNode;
  paymentDetailsForm: React.ReactNode;
};

export const CreateSubscriptionSchema = Yup.object().shape({
  name: Yup.string()
    .min(1, "Billing contact name must be at least 1 character long.")
    .max(128, "Billing contact name must be at most 128 characters long.")
    .required("Billing contact name is required."),
  email: Yup.string()
    .email("Invalid email address.")
    .min(1, "Billing contact name must be at least 1 character long.")
    .required("Billing contact email is required."),
});

export function UpgradePlanContentContainer({
  team,
  email: profileEmail,
  name: profileName,
  onUpgradeComplete,
  plan,
  ...props
}: Pick<UpgradePlanContentProps, "numMembers" | "plan"> & {
  team: Team;
  email?: string;
  name?: string | null;
  onUpgradeComplete: () => void;
}) {
  const createSubscription = useCreateSubscription(team.id);
  const { spendingLimits } = useLaunchDarkly();

  const formState = useFormik<UpgradeFormState>({
    initialValues: {
      promoCode: "",
      name: profileName || "",
      email: profileEmail || "",
      planId: plan.id,
      paymentMethod: undefined,
      billingAddress: undefined,
      spendingLimitWarningThresholdUsd: undefined,
      spendingLimitDisableThresholdUsd: null,
    },
    validationSchema: spendingLimits
      ? CreateSubscriptionSchema.concat(
          spendingLimitsSchema({
            // A new billing cycle starts when the user upgrades, so we don’t need to show the
            // warning about setting a spending limit lower than the amount spent in the current
            // billing cycle.
            currentSpending: undefined,
          }),
        )
      : CreateSubscriptionSchema,
    onSubmit: async (v) => {
      await createSubscription({
        planId: v.planId,
        paymentMethod: v.paymentMethod,
        billingAddress: v.billingAddress,
        name: v.name,
        email: v.email,
        ...(spendingLimits && {
          warningThresholdCents:
            typeof v.spendingLimitWarningThresholdUsd === "number"
              ? v.spendingLimitWarningThresholdUsd * 100
              : v.spendingLimitWarningThresholdUsd,
          disableThresholdCents:
            typeof v.spendingLimitDisableThresholdUsd === "number"
              ? v.spendingLimitDisableThresholdUsd * 100
              : v.spendingLimitDisableThresholdUsd,
        }),
      });
      onUpgradeComplete();
    },
  });
  const [debouncedPromoCode, setDebouncedPromoCode] = useState(
    formState.values.promoCode,
  );
  useDebounce(
    () => {
      setDebouncedPromoCode(formState.values.promoCode);
    },
    debounceDurationMs,
    [formState.values.promoCode],
  );

  const couponData = useGetCoupon(team.id, plan.id, debouncedPromoCode || "");

  useEffect(() => {
    void formState.setFieldValue(
      "planId",
      couponData.coupon?.planId || plan.id,
    );
    // Don't need setFieldValue in deps
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [couponData.coupon, plan.id]);

  const setPaymentMethod = useCallback(
    async (method?: string) => {
      await formState.setFieldValue("paymentMethod", method);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const setBillingAddress = useCallback(
    async (address?: Address) => {
      await formState.setFieldValue("billingAddress", address);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const {
    stripePromise,
    options,
    retrieveSetupIntent,
    confirmSetup,
    resetClientSecret,
  } = useStripePaymentSetup(
    team,
    formState.values.paymentMethod ?? undefined,
    setPaymentMethod,
  );

  return (
    <FormikProvider value={formState}>
      <UpgradePlanContent
        {...props}
        plan={plan}
        setPaymentMethod={(p) => {
          if (!p) {
            resetClientSecret();
          }
          void formState.setFieldValue("paymentMethod", p);
        }}
        teamMemberDiscountPct={couponData.coupon?.percentOff}
        requiresPaymentMethod={
          couponData.coupon ? couponData.coupon.requiresPaymentMethod : true
        }
        couponDurationInMonths={
          couponData.coupon?.durationInMonths ?? undefined
        }
        isLoadingPromo={couponData.isLoading}
        promoCodeError={couponData.errorMessage}
        billingAddressInputs={
          options.clientSecret ? (
            <div className="flex flex-col gap-2">
              <h4>Billing Address</h4>
              <Elements stripe={stripePromise} options={options}>
                <BillingAddressInputs onChangeAddress={setBillingAddress} />
              </Elements>
            </div>
          ) : undefined
        }
        paymentDetailsForm={
          // Using dependency injection to pass in the Stripe form
          // so we can test the UpgradePlanContent component in isolation
          options.clientSecret ? (
            <Elements stripe={stripePromise} options={options}>
              <PaymentDetailsForm
                retrieveSetupIntent={retrieveSetupIntent}
                confirmSetup={confirmSetup}
              />
            </Elements>
          ) : undefined
        }
      />
    </FormikProvider>
  );
}

export function UpgradePlanContent({
  plan,
  couponDurationInMonths,
  teamMemberDiscountPct = 0,
  numMembers,
  requiresPaymentMethod = true,
  isLoadingPromo = false,
  promoCodeError,
  setPaymentMethod,
  billingAddressInputs,
  paymentDetailsForm,
}: UpgradePlanContentProps) {
  const formState = useFormikContext<UpgradeFormState>();
  const { spendingLimits } = useLaunchDarkly();

  if (teamMemberDiscountPct < 0 || teamMemberDiscountPct > 1) {
    throw new Error(
      `Invalid teamMemberDiscountPct: ${teamMemberDiscountPct}. Must be between 0 and 1.`,
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <PriceSummary
        plan={plan}
        teamMemberDiscountPct={teamMemberDiscountPct}
        numMembers={numMembers}
        requiresPaymentMethod={requiresPaymentMethod}
        couponDurationInMonths={couponDurationInMonths}
      />
      <div className="flex max-w-64 items-center gap-2">
        <TextInput
          label="Promo code"
          placeholder="Enter a promo code"
          onChange={(e) =>
            formState.setFieldValue("promoCode", e.target.value.toUpperCase())
          }
          value={formState.values.promoCode}
          id="promoCode"
          error={promoCodeError}
        />
        {isLoadingPromo && (
          <span data-testid="loading-spinner" className="mt-4">
            <Spinner />
          </span>
        )}
      </div>

      <div className="flex flex-col gap-2">
        <h4>Billing Contact</h4>
        <BillingContactInputs formState={formState} />
      </div>

      {billingAddressInputs}

      {spendingLimits && (
        <div className="flex flex-col gap-2">
          <h4>Usage Spending Limits</h4>
          <SpendingLimits />
        </div>
      )}

      {requiresPaymentMethod && !formState.values.paymentMethod && (
        <div className="flex flex-col gap-2">
          <h4>Payment Details</h4>
          {/* Reduce the amount of space stripe can take up so it doesn't render horizontal overflow */}
          <div className="max-w-[calc(100%-1rem)]">{paymentDetailsForm}</div>
        </div>
      )}

      {(!requiresPaymentMethod || formState.values.paymentMethod) && (
        <form
          className="mt-2 flex flex-col items-start gap-4 text-sm"
          onSubmit={async (e) => {
            e.preventDefault();
            await formState.handleSubmit();
          }}
        >
          <div className="flex gap-4">
            {formState.values.paymentMethod && (
              <Button
                size="sm"
                onClick={() => setPaymentMethod(undefined)}
                data-testid="update-payment-method-button"
                variant="neutral"
              >
                Change payment method
              </Button>
            )}
            <Button
              data-testid="upgrade-plan-button"
              className="w-fit"
              size="sm"
              disabled={
                isLoadingPromo ||
                (requiresPaymentMethod && !formState.values.paymentMethod) ||
                !formState.values.billingAddress ||
                !formState.isValid ||
                formState.isSubmitting
              }
              type="submit"
              icon={formState.isSubmitting && <Spinner />}
              tip={
                requiresPaymentMethod && !formState.values.paymentMethod
                  ? "Add a payment method to continue."
                  : !formState.values.name
                    ? "Enter a billing contact name to continue."
                    : !formState.values.email
                      ? "Enter a billing contact email to continue."
                      : !formState.values.billingAddress
                        ? "Enter a billing address to continue."
                        : undefined
              }
            >
              Confirm and Upgrade
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}

export function PriceInDollars({
  price,
  percentOff,
}: {
  price: number;
  percentOff: number;
}) {
  return percentOff ? (
    <>
      <span className="mr-1 line-through">${price}</span>
      <span className="font-semibold">
        ${Number((price * (1 - percentOff)).toFixed(2))}
      </span>
    </>
  ) : (
    <span className="font-semibold">${price}</span>
  );
}

function PriceSummary({
  plan,
  teamMemberDiscountPct,
  numMembers,
  couponDurationInMonths,
  requiresPaymentMethod,
}: {
  plan: PlanResponse;
  teamMemberDiscountPct: number;
  numMembers: number;
  couponDurationInMonths?: number;
  requiresPaymentMethod: boolean;
}) {
  return (
    <div className="flex flex-col gap-2 text-sm" data-testid="price-summary">
      <p>
        The {plan.name} plan costs{" "}
        <PriceInDollars
          price={plan.seatPrice}
          percentOff={!requiresPaymentMethod ? 1 : teamMemberDiscountPct}
        />{" "}
        per team member, per month.
      </p>
      {couponDurationInMonths && (
        <p>
          This discount will be applied for the next {couponDurationInMonths}{" "}
          months.
        </p>
      )}
      {requiresPaymentMethod && (
        <p>
          Your team has {numMembers} member{numMembers > 1 && "s"}. Once you
          upgrade, you'll be charged{" "}
          <PriceInDollars
            price={plan.seatPrice * numMembers}
            percentOff={teamMemberDiscountPct}
          />{" "}
          immediately.{" "}
        </p>
      )}
    </div>
  );
}
