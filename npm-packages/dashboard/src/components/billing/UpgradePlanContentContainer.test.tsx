import { render, waitFor } from "@testing-library/react";
import { fireEvent, screen } from "@testing-library/dom";
import { PlanResponse, TeamResponse } from "generatedApi";
import React from "react";
import { UpgradePlanContentContainer } from "./UpgradePlanContent";

const team: TeamResponse = {
  id: 1,
  creator: 1,
  slug: "team",
  name: "Team",
  suspended: false,
  referralCode: "CODE123",
};

const email = "nicolas@convex.dev";
const name = "Nicolas";

const onUpgradeComplete = jest.fn();

jest.mock("@stripe/react-stripe-js", () => ({
  Elements: ({ children }: React.PropsWithChildren) => children,
}));

jest.mock("./BillingAddressInputs", () => ({
  BillingAddressInputs: ({
    onChangeAddress,
  }: {
    onChangeAddress(address: any): Promise<void>;
  }) => (
    <div className="flex flex-col gap-2">
      <input
        type="text"
        placeholder="Billing Address"
        data-testid="mock-billing-address"
        onChange={(e) => {
          void onChangeAddress({
            line1: e.target.value,
            city: "Test City",
            state: "CA",
            postal_code: "12345",
            country: "US",
          });
        }}
      />
    </div>
  ),
}));

const mockCreateSubscription = jest.fn().mockResolvedValue(undefined);
jest.mock("api/billing", () => ({
  useCreateSubscription: () => mockCreateSubscription,
  useGetCoupon: jest.fn().mockReturnValue({
    isLoading: false,
    coupon: {
      requiresPaymentMethod: false,
    },
    errorMessage: null,
  }),
}));

const mockPlan: PlanResponse = {
  name: "Basic",
  id: "basic",
  planType: "basic",
  seatPrice: 10,
  description: "abc",
  status: "active",
};

jest.mock("hooks/useStripe", () => ({
  useStripePaymentSetup: jest.fn().mockImplementation(() => ({
    options: {
      clientSecret: "123",
    },
  })),
}));

describe("UpgradePlanContentContainer", () => {
  it("can submit the form with spending limits", async () => {
    render(
      <UpgradePlanContentContainer
        team={team}
        email={email}
        name={name}
        onUpgradeComplete={onUpgradeComplete}
        numMembers={0}
        plan={mockPlan}
        isChef={false}
      />,
    );

    // Fill in the billing address
    const billingAddressInput = screen.getByTestId("mock-billing-address");
    expect(billingAddressInput).toBeInTheDocument();
    fireEvent.change(billingAddressInput, { target: { value: "123 Main St" } });

    // Fill in the spending limit
    const spendingLimitWarningThresholdUsdInput =
      screen.getByLabelText("Warning Threshold");
    expect(spendingLimitWarningThresholdUsdInput).toBeInTheDocument();
    fireEvent.change(spendingLimitWarningThresholdUsdInput, {
      target: { value: "100" },
    });

    // Submit
    const upgradeButton = screen.getByTestId("upgrade-plan-button");
    expect(upgradeButton).toBeInTheDocument();
    expect(upgradeButton).not.toBeDisabled();
    fireEvent.click(upgradeButton);

    // Verify the form submission
    await waitFor(() => {
      expect(mockCreateSubscription).toHaveBeenCalledWith({
        planId: "basic",
        paymentMethod: undefined,
        billingAddress: {
          line1: "123 Main St",
          city: "Test City",
          state: "CA",
          postal_code: "12345",
          country: "US",
        },
        name: "Nicolas",
        email: "nicolas@convex.dev",
        warningThresholdCents: 100_00,
        disableThresholdCents: null,
      });
      expect(onUpgradeComplete).toHaveBeenCalled();
    });
  });
});
