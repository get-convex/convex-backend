import {
  useListInvoices,
  useUpdateBillingAddress,
  useUpdateBillingContact,
  useUpdatePaymentMethod,
  useResumeSubscription,
} from "api/billing";
import { Loading, Button, Spinner, formatDate, Sheet } from "dashboard-common";
import { useFormik } from "formik";
import { useStripeAddressSetup, useStripePaymentSetup } from "hooks/useStripe";
import { Elements } from "@stripe/react-stripe-js";
import { useCallback, useRef, useState } from "react";
import { useMount } from "react-use";
import {
  Address,
  BillingContactResponse,
  OrbSubscriptionResponse,
  Team,
} from "generatedApi";
import { BillingContactInputs } from "./BillingContactInputs";
import { CreateSubscriptionSchema } from "./UpgradePlanContent";
import { PaymentDetailsForm } from "./PaymentDetailsForm";
import { Invoices } from "./Invoices";
import { BillingAddressInputs } from "./BillingAddressInputs";

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
          <BillingContactForm
            subscription={subscription}
            team={team}
            hasAdminPermissions={hasAdminPermissions}
          />
          <BillingAddressForm
            subscription={subscription}
            team={team}
            hasAdminPermissions={hasAdminPermissions}
          />
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

function BillingContactForm({
  subscription,
  team,
  hasAdminPermissions,
}: {
  subscription: OrbSubscriptionResponse;
  team: Team;
  hasAdminPermissions: boolean;
}) {
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
    },
    enableReinitialize: true,
  });

  return (
    <form
      className="w-fit"
      onSubmit={(e) => {
        e.preventDefault();
        formState.handleSubmit();
      }}
    >
      <BillingContactInputs
        formState={formState}
        disabled={!hasAdminPermissions}
      />
      <div className="mt-4 gap-2">
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
      </div>
    </form>
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
  const ref = useRef<HTMLFormElement>(null);
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
    <form
      className="w-fit"
      onSubmit={(e) => {
        e.preventDefault();
        formState.handleSubmit();
      }}
      ref={ref}
    >
      {hasAdminPermissions ? (
        options.clientSecret ? (
          <Elements stripe={stripePromise} options={options}>
            <BillingAddressInputs
              onChangeAddress={setBillingAddress}
              existingBillingAddress={subscription.billingAddress || undefined}
              name={subscription.billingContact.name}
            />
          </Elements>
        ) : null
      ) : (
        <div className="flex flex-col gap-4">
          <h4>Billing Address</h4>
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

      <div className="mt-4 gap-2">
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
          {hasAdminPermissions
            ? formState.isSubmitting
              ? "Saving Billing Address..."
              : "Save Billing Address"
            : "Change Billing Address"}
        </Button>
      </div>
    </form>
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
