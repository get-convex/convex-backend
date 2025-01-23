import { TextInput } from "dashboard-common";
import { useFormik } from "formik";
import { BillingContactResponse, CreateSubscriptionArgs } from "generatedApi";

export function BillingContactInputs({
  formState,
  disabled = false,
}: {
  formState:
    | ReturnType<
        typeof useFormik<CreateSubscriptionArgs & { promoCode?: string }>
      >
    | ReturnType<typeof useFormik<BillingContactResponse>>;
  disabled?: boolean;
}) {
  return (
    <div className="flex flex-col gap-2">
      <h4>Billing Contact</h4>
      <div className="flex flex-wrap gap-4">
        <div>
          <TextInput
            label="Name"
            placeholder="Billing contact name"
            outerClassname="w-64"
            error={formState.errors.name}
            onChange={formState.handleChange}
            value={formState.values.name}
            id="name"
            disabled={disabled}
          />
        </div>
        <div>
          <TextInput
            label="Email"
            error={formState.errors.email}
            placeholder="Billing contact email"
            outerClassname="w-64"
            onChange={formState.handleChange}
            value={formState.values.email}
            id="email"
            disabled={disabled}
          />
        </div>
      </div>
    </div>
  );
}
