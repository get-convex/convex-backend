import {
  useListInvoices,
  useUpdateBillingAddress,
  useUpdateBillingContact,
  useUpdatePaymentMethod,
  useResumeSubscription,
  useGetCurrentSpend,
  useGetSpendingLimits,
} from "api/billing";
import { Loading } from "@ui/Loading";
import { Button } from "@ui/Button";
import { Spinner } from "@ui/Spinner";
import { formatDate } from "@common/lib/format";
import { Sheet } from "@ui/Sheet";
import { useFormik } from "formik";
import { useStripeAddressSetup, useStripePaymentSetup } from "hooks/useStripe";
import { Elements } from "@stripe/react-stripe-js";
import { useCallback, useMemo, useRef, useState } from "react";
import { useMount } from "react-use";
import {
  Address,
  BillingContactResponse,
  OrbSubscriptionResponse,
  Team,
} from "generatedApi";
import { Tooltip } from "@ui/Tooltip";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { Callout } from "@ui/Callout";
import { formatUsd } from "@common/lib/utils";
import { BillingContactInputs } from "./BillingContactInputs";
import { CreateSubscriptionSchema } from "./UpgradePlanContent";
import { PaymentDetailsForm } from "./PaymentDetailsForm";
import { Invoices } from "./Invoices";
import { BillingAddressInputs } from "./BillingAddressInputs";
import {
  SpendingLimitsForm,
  SpendingLimitsValue,
  useSubmitSpendingLimits,
} from "./SpendingLimits";
import { useLaunchDarkly } from "../../hooks/useLaunchDarkly";

export function SubscriptionOverview({
  team,
  hasAdminPermissions,
  subscription,
}: {
  team: Team;
  hasAdminPermissions: boolean;
  subscription?: OrbSubscriptionResponse | null;
}) {
  const isLoading = subscription === undefined;
  const resumeSubscription = useResumeSubscription(team.id);
  const [isResuming, setIsResuming] = useState(false);
  const { invoices, isLoading: isLoadingInvoices } = useListInvoices(team.id);
  const { spendingLimits } = useLaunchDarkly();

  if (isLoading || isLoadingInvoices) {
    return <Loading className="h-60 w-full" fullHeight={false} />;
  }
  const nextInvoiceDate = invoices?.find(
    (i) => i.status === "draft",
  )?.invoiceDate;
  return (
    <>
      {subscription && (
        <Sheet className="flex flex-col gap-4">
          <h3>Subscription</h3>
          <div className="text-sm">
            Current Plan:{" "}
            <span className="font-semibold">{subscription.plan.name}</span>
          </div>
          {typeof subscription.endDate === "number" ? (
            <>
              <div className="text-sm">
                Subscription ends on{" "}
                <span className="font-semibold">
                  {formatDate(new Date(subscription.endDate))}
                </span>
              </div>
              <Button
                disabled={!hasAdminPermissions || isResuming}
                className="w-fit"
                tip={
                  !hasAdminPermissions &&
                  "You do not have permission to modify the team subscription."
                }
                icon={isResuming ? <Spinner /> : null}
                onClick={async () => {
                  setIsResuming(true);
                  try {
                    await resumeSubscription();
                  } finally {
                    setIsResuming(false);
                  }
                }}
              >
                Resume Subscription
              </Button>
            </>
          ) : typeof nextInvoiceDate === "number" ? (
            <div className="text-sm">
              Subscription renews on{" "}
              <span className="font-semibold">
                {formatDate(new Date(nextInvoiceDate))}
              </span>
            </div>
          ) : null}
          <hr />
          {spendingLimits && (
            <>
              <SpendingLimitsSectionContainer
                subscription={subscription}
                team={team}
                hasAdminPermissions={hasAdminPermissions}
              />
              <hr />
            </>
          )}
          <BillingContactForm
            subscription={subscription}
            team={team}
            hasAdminPermissions={hasAdminPermissions}
          />
          <hr />
          <BillingAddressForm
            subscription={subscription}
            team={team}
            hasAdminPermissions={hasAdminPermissions}
          />
          <hr />
          <PaymentMethodForm
            subscription={subscription}
            team={team}
            hasAdminPermissions={hasAdminPermissions}
          />
        </Sheet>
      )}
      {invoices && (invoices.length > 0 || subscription) && (
        <Invoices invoices={invoices} />
      )}
    </>
  );
}

function SpendingLimitsSectionContainer({
  subscription,
  team,
  hasAdminPermissions,
}: {
  subscription: OrbSubscriptionResponse;
  team: Team;
  hasAdminPermissions: boolean;
}) {
  const submitSpendingLimits = useSubmitSpendingLimits(team);

  const { totalCents } = useGetCurrentSpend(
    hasAdminPermissions ? team.id : null,
  );
  const currentSpend = useMemo(() => {
    if (
      totalCents === undefined ||
      subscription.nextBillingPeriodStart === undefined
    ) {
      return undefined;
    }

    return {
      totalCents,
      nextBillingPeriodStart: subscription.nextBillingPeriodStart,
    };
  }, [totalCents, subscription.nextBillingPeriodStart]);

  const { spendingLimits } = useGetSpendingLimits(team.id);

  return (
    <SpendingLimitsSection
      currentSpendLimit={spendingLimits}
      currentSpend={currentSpend}
      hasAdminPermissions={hasAdminPermissions}
      onSubmit={submitSpendingLimits}
    />
  );
}

export function SpendingLimitsSection({
  currentSpendLimit,
  currentSpend,
  hasAdminPermissions,
  onSubmit,
}: {
  currentSpendLimit:
    | {
        disableThresholdCents: number | null;
        warningThresholdCents: number | null;
        state: null | "Running" | "Disabled" | "Warning";
      }
    | undefined;
  currentSpend:
    | { totalCents: number; nextBillingPeriodStart: string }
    | undefined;
  hasAdminPermissions: boolean;
  onSubmit: (v: SpendingLimitsValue) => Promise<void>;
}) {
  const [showForm, setShowForm] = useState(false);

  return (
    <div className="flex flex-col gap-4">
      <h4>Usage Spending Limits</h4>

      {currentSpendLimit?.state === "Disabled" && (
        <Callout variant="error">
          Your projects are disabled because you exceeded your spending limit.
          Increase it to re-enable your projects.
        </Callout>
      )}

      {!showForm ? (
        <>
          <div className="flex flex-wrap gap-x-8 gap-y-4">
            {currentSpendLimit === undefined ? (
              <>
                <Loading className="h-12 w-36" fullHeight={false} />
                <Loading className="h-12 w-36" fullHeight={false} />
              </>
            ) : currentSpendLimit.disableThresholdCents === null &&
              currentSpendLimit.warningThresholdCents === null ? (
              <p>You don’t have any spending limits set.</p>
            ) : (
              <>
                {currentSpendLimit.warningThresholdCents !== null && (
                  <CostLabel
                    label="Warning threshold"
                    priceCents={currentSpendLimit.warningThresholdCents}
                    tooltip="If your usage exceeds this amount, admins in your team will be notified by email."
                  />
                )}
                {currentSpendLimit.disableThresholdCents !== null && (
                  <CostLabel
                    label="Disable threshold"
                    priceCents={currentSpendLimit.disableThresholdCents}
                    tooltip={`If your usage exceeds ${currentSpendLimit.disableThresholdCents === 0 ? "the built-in limits of your plan" : "this amount"}, all your projects will be paused.`}
                  />
                )}
              </>
            )}
          </div>

          <Button
            className="w-fit"
            onClick={() => setShowForm(true)}
            variant="neutral"
            disabled={!hasAdminPermissions}
            tip={
              !hasAdminPermissions &&
              "You do not have permission to change your spending limits"
            }
          >
            {currentSpendLimit === null
              ? "Set spending limits"
              : "Change spending limits"}
          </Button>
        </>
      ) : (
        <SpendingLimitsForm
          defaultValue={
            currentSpendLimit === undefined
              ? undefined
              : {
                  spendingLimitWarningThresholdUsd:
                    currentSpendLimit.warningThresholdCents === null
                      ? null
                      : currentSpendLimit.warningThresholdCents / 100,
                  spendingLimitDisableThresholdUsd:
                    currentSpendLimit.disableThresholdCents === null
                      ? null
                      : currentSpendLimit.disableThresholdCents / 100,
                }
          }
          currentSpending={currentSpend}
          onSubmit={async (v) => {
            await onSubmit(v);
            setShowForm(false);
          }}
          onCancel={() => setShowForm(false)}
        />
      )}
    </div>
  );
}

function CostLabel({
  label,
  priceCents,
  tooltip,
}: {
  label: string;
  priceCents: number;
  tooltip: string;
}) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="flex items-center gap-1 text-content-secondary">
        {label}
        <Tooltip tip={tooltip} side="top">
          <QuestionMarkCircledIcon className="text-content-tertiary" />
        </Tooltip>
      </span>
      <span className="flex items-baseline gap-1">
        {/* eslint-disable-next-line no-restricted-syntax */}
        <div className="text-lg font-medium">{formatUsd(priceCents / 100)}</div>
        <span className="text-sm text-content-secondary">/ month</span>
      </span>
    </div>
  );
}

function BillingContactForm({
  subscription,
  team,
  hasAdminPermissions,
}: {
  subscription: OrbSubscriptionResponse;
  team: Team;
  hasAdminPermissions: boolean;
}) {
  const [showForm, setShowForm] = useState(false);
  const updateBillingContact = useUpdateBillingContact(team.id);
  const formState = useFormik<BillingContactResponse>({
    initialValues: {
      name: subscription.billingContact.name,
      email: subscription.billingContact.email,
    },
    validationSchema: CreateSubscriptionSchema,
    onSubmit: async (v) => {
      await updateBillingContact(v);
      await formState.setTouched({});
      setShowForm(false);
    },
    enableReinitialize: true,
  });

  return (
    <div className="flex flex-col gap-4">
      <h4>Billing Contact</h4>
      {!showForm ? (
        <>
          <div className="text-sm">
            <div>
              <span className="font-semibold">
                {subscription.billingContact.name}
              </span>
            </div>
            <div>{subscription.billingContact.email}</div>
          </div>
          <Button
            className="w-fit"
            onClick={() => setShowForm(true)}
            variant="neutral"
            disabled={!hasAdminPermissions}
            tip={
              !hasAdminPermissions &&
              "You do not have permission to update the billing contact"
            }
          >
            Change billing contact
          </Button>
        </>
      ) : (
        <form
          className="max-w-64"
          onSubmit={(e) => {
            e.preventDefault();
            formState.handleSubmit();
          }}
        >
          <BillingContactInputs
            formState={formState}
            disabled={!hasAdminPermissions}
          />
          <div className="mt-4 flex gap-2">
            <Button
              type="submit"
              disabled={
                !formState.dirty ||
                !formState.isValid ||
                formState.isSubmitting ||
                !hasAdminPermissions
              }
              tip={
                !hasAdminPermissions &&
                "You do not have permission to update the billing contact"
              }
              icon={formState.isSubmitting ? <Spinner /> : null}
            >
              {formState.isSubmitting
                ? "Saving Billing Contact..."
                : "Save Billing Contact"}
            </Button>
            <Button
              type="button"
              variant="neutral"
              onClick={() => {
                formState.resetForm();
                setShowForm(false);
              }}
            >
              Cancel
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}

function BillingAddressForm({
  team,
  subscription,
  hasAdminPermissions,
}: {
  team: Team;
  subscription: OrbSubscriptionResponse;
  hasAdminPermissions: boolean;
}) {
  const [showForm, setShowForm] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useMount(() => {
    window.location.hash === "#billingAddress" && ref.current?.scrollIntoView();
  });

  const updateBillingAddress = useUpdateBillingAddress(team.id);
  const formState = useFormik<{ billingAddress?: Address }>({
    initialValues: {
      billingAddress: subscription.billingAddress || undefined,
    },
    onSubmit: async (v) => {
      if (v.billingAddress) {
        await updateBillingAddress({ billingAddress: v.billingAddress });
        await formState.setTouched({});
        setShowForm(false);
      }
    },
    enableReinitialize: true,
  });

  const { setFieldValue } = formState;
  const setBillingAddress = useCallback(
    async (address?: Address) => {
      await setFieldValue("billingAddress", address);
    },
    [setFieldValue],
  );
  const { stripePromise, options } = useStripeAddressSetup(
    team,
    hasAdminPermissions,
  );

  return (
    <div className="flex flex-col gap-4" ref={ref}>
      <h4>Billing Address</h4>
      {!showForm ? (
        <>
          <div className="text-sm">
            {subscription.billingAddress ? (
              <div>
                <div>
                  {subscription.billingAddress.line1}
                  {subscription.billingAddress.line2 && (
                    <div>{subscription.billingAddress.line2}</div>
                  )}
                  <div>
                    {subscription.billingAddress.city},{" "}
                    {subscription.billingAddress.state}{" "}
                    {subscription.billingAddress.postal_code}
                  </div>
                  <div>{subscription.billingAddress.country}</div>
                </div>
              </div>
            ) : (
              <div>No billing address on file.</div>
            )}
          </div>
          <Button
            className="w-fit"
            onClick={() => setShowForm(true)}
            disabled={!hasAdminPermissions}
            variant="neutral"
            tip={
              !hasAdminPermissions &&
              "You do not have permission to update the billing address"
            }
          >
            {subscription.billingAddress
              ? "Change billing address"
              : "Add billing address"}
          </Button>
        </>
      ) : (
        <form
          className="w-full"
          onSubmit={(e) => {
            e.preventDefault();
            formState.handleSubmit();
          }}
        >
          {hasAdminPermissions ? (
            options.clientSecret ? (
              <Elements stripe={stripePromise} options={options}>
                <BillingAddressInputs
                  onChangeAddress={setBillingAddress}
                  existingBillingAddress={
                    subscription.billingAddress || undefined
                  }
                  name={subscription.billingContact.name}
                />
              </Elements>
            ) : null
          ) : (
            <div className="flex flex-col gap-4">
              <div className="text-sm">
                {subscription.billingAddress ? (
                  <div>
                    <div>
                      {subscription.billingAddress.line1}
                      {subscription.billingAddress.line2 && (
                        <div>{subscription.billingAddress.line2}</div>
                      )}
                      <div>
                        {subscription.billingAddress.city},{" "}
                        {subscription.billingAddress.state}{" "}
                        {subscription.billingAddress.postal_code}
                      </div>
                      <div>{subscription.billingAddress.country}</div>
                    </div>
                  </div>
                ) : (
                  <div>No billing address on file.</div>
                )}
              </div>
            </div>
          )}

          <div className="mt-4 flex gap-2">
            <Button
              type="submit"
              disabled={
                !formState.dirty ||
                !formState.values.billingAddress ||
                formState.isSubmitting ||
                !hasAdminPermissions
              }
              tip={
                !hasAdminPermissions &&
                "You do not have permission to update the billing address"
              }
              icon={formState.isSubmitting ? <Spinner /> : null}
            >
              {formState.isSubmitting
                ? "Saving Billing Address..."
                : "Save Billing Address"}
            </Button>
            <Button
              type="button"
              variant="neutral"
              onClick={() => {
                formState.resetForm();
                setShowForm(false);
              }}
            >
              Cancel
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}

function PaymentMethodForm({
  team,
  subscription,
  hasAdminPermissions,
}: {
  team: Team;
  subscription: OrbSubscriptionResponse;
  hasAdminPermissions: boolean;
}) {
  const [showForm, setShowForm] = useState(false);
  const onSave = useCallback(() => {
    setShowForm(false);
  }, []);

  const ref = useRef<HTMLDivElement>(null);
  useMount(() => {
    window.location.hash === "#paymentMethod" && ref.current?.scrollIntoView();
  });

  return (
    <div className="flex flex-col gap-4">
      <h4>Payment Method</h4>
      {subscription.paymentMethod && (
        <div className="text-sm">
          Current payment method:{" "}
          <span className="font-semibold">
            {subscription.paymentMethod.display}
          </span>
        </div>
      )}
      {showForm ? (
        <UpdatePaymentMethod team={team} onSave={onSave} />
      ) : (
        <Button
          ref={ref}
          className="w-fit"
          onClick={() => setShowForm(true)}
          disabled={!hasAdminPermissions}
          variant="neutral"
          tip={
            !hasAdminPermissions &&
            "You do not have permission to update the payment method"
          }
        >
          {subscription.paymentMethod
            ? "Change payment method"
            : "Add payment method"}
        </Button>
      )}
    </div>
  );
}
function UpdatePaymentMethod({
  team,
  onSave,
}: {
  team: Team;
  onSave: () => void;
}) {
  const updatePaymentMethod = useUpdatePaymentMethod(team.id);
  const updatePaymentMethodCb = useCallback(
    async (paymentMethod?: string) => {
      if (paymentMethod) {
        await updatePaymentMethod({ paymentMethod });
        onSave();
      }
    },
    [onSave, updatePaymentMethod],
  );
  const { stripePromise, options, retrieveSetupIntent, confirmSetup } =
    useStripePaymentSetup(team, undefined, updatePaymentMethodCb);

  return options.clientSecret ? (
    <Elements stripe={stripePromise} options={options}>
      <PaymentDetailsForm
        retrieveSetupIntent={retrieveSetupIntent}
        confirmSetup={confirmSetup}
      />
    </Elements>
  ) : null;
}
