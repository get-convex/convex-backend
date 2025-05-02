import React, { useEffect, useState } from "react";
import {
  PaymentElement,
  useElements,
  useStripe,
} from "@stripe/react-stripe-js";
import {
  StripePaymentElementOptions,
  StripeElements,
  SetupIntent,
} from "@stripe/stripe-js";
import { Button } from "@ui/Button";

export function PaymentDetailsForm({
  retrieveSetupIntent,
  confirmSetup,
}: {
  retrieveSetupIntent: () => Promise<SetupIntent | null>;
  confirmSetup: (
    elements: StripeElements,
  ) => Promise<{ error?: string; paymentMethod?: string } | undefined>;
}) {
  const stripe = useStripe();
  const elements = useElements();

  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    void retrieveSetupIntent();
  }, [retrieveSetupIntent]);

  const paymentElementOptions: StripePaymentElementOptions = {
    layout: "tabs",
  };

  const [error, setError] = useState<string>();

  return (
    <form
      className="flex w-full flex-col gap-4"
      onSubmit={async (e) => {
        e.preventDefault();
        if (!elements) {
          return;
        }
        setIsLoading(true);
        const confirmResult = await confirmSetup(elements);
        if (confirmResult === undefined) {
          setIsLoading(false);
          return;
        }
        if (confirmResult.error) {
          setError(confirmResult.error);
        }
        setIsLoading(false);
      }}
    >
      <PaymentElement
        options={paymentElementOptions}
        onChange={() => error !== undefined && setError(undefined)}
      />
      <Button
        type="submit"
        className="w-fit"
        disabled={!stripe || !elements}
        loading={isLoading}
        size="sm"
      >
        Save payment method
      </Button>
      {error && (
        <p className="h-fit text-xs text-content-errorSecondary" role="alert">
          {error}
        </p>
      )}
    </form>
  );
}
