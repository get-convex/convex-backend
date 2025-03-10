import {
  render,
  screen,
  fireEvent,
  renderHook,
  waitFor,
} from "@testing-library/react";
import { FormikProvider, useFormik } from "formik";
import { act } from "react";
import { CreateSubscriptionArgs, PlanResponse } from "generatedApi";
import {
  CreateSubscriptionSchema,
  PriceInDollars,
  UpgradePlanContent,
  UpgradePlanContentProps,
} from "./UpgradePlanContent";

jest.mock("api/billing", () => {});

describe("UpgradePlanContent", () => {
  const mockPlan: PlanResponse = {
    name: "Basic",
    id: "basic",
    planType: "basic",
    seatPrice: 10,
    description: "abc",
    status: "active",
  };
  const mockNumMembers = 5;
  const mockOnCreateSubscription = jest.fn();
  function newFormState(
    initialValues?: Partial<CreateSubscriptionArgs & { promoCode?: string }>,
  ) {
    return renderHook(() =>
      useFormik<CreateSubscriptionArgs & { promoCode?: string }>({
        initialValues: {
          promoCode: "",
          name: "",
          email: "",
          planId: "123",
          paymentMethod: undefined,
          ...initialValues,
        },
        validationSchema: CreateSubscriptionSchema,
        onSubmit: (v) => {
          mockOnCreateSubscription(v);
        },
      }),
    );
  }
  let mockFormState = newFormState();

  beforeEach(() => {
    jest.resetAllMocks();
    mockFormState = newFormState();
  });

  function renderUI(props?: Partial<UpgradePlanContentProps>) {
    render(
      <FormikProvider value={mockFormState.result.current}>
        <UpgradePlanContent
          plan={mockPlan}
          numMembers={mockNumMembers}
          setPaymentMethod={jest.fn()}
          billingAddressInputs={null}
          paymentDetailsForm={null} // Add the required paymentDetailsForm property
          {...props}
        />
      </FormikProvider>,
    );
  }

  it("renders the price summary", async () => {
    renderUI();
    const priceSummaryElement = screen.getByTestId("price-summary");
    await waitFor(() => expect(priceSummaryElement).toBeInTheDocument());
  });

  it("does not render the payment method button when the plan is free", async () => {
    renderUI({ teamMemberDiscountPct: 1 });
    const paymentMethodElement = screen.queryByTestId(
      "update-payment-method-button",
    );
    await waitFor(() => expect(paymentMethodElement).not.toBeInTheDocument());
  });

  it("renders the promo code input", async () => {
    renderUI();
    const promoCodeInput = screen.getByLabelText("Promo code");
    await waitFor(() => expect(promoCodeInput).toBeInTheDocument());
  });

  it("renders the promo code spinner while loading", () => {
    renderUI({ isLoadingPromo: true });
    screen.getByTestId("loading-spinner");
  });

  it("renders the promo code error", () => {
    const promoCodeError = "Invalid promo code";
    renderUI({ promoCodeError });
    screen.getByText(promoCodeError);
  });

  it("calls onCreateSubscription when upgrade plan button is clicked", async () => {
    mockFormState = newFormState({
      promoCode: "",
      name: "Ari",
      email: "ari@convex.dev",
      planId: "123",
      paymentMethod: "wah",
      billingAddress: {
        line1: "123 Main St",
        line2: null,
        city: "San Francisco",
        state: "CA",
        postal_code: "94105",
        country: "US",
      },
    });
    renderUI();
    act(() => {
      const upgradePlanButton = screen.getByTestId("upgrade-plan-button");
      expect(upgradePlanButton).not.toBeDisabled();
      fireEvent.click(upgradePlanButton);
    });
    await waitFor(() => {
      expect(mockOnCreateSubscription).toHaveBeenCalledWith(
        mockFormState.result.current.values,
      );
    });
  });
});

describe("PriceInDollars", () => {
  it("renders the price", () => {
    const price = 100;
    const percentOff = 0;
    const { container } = render(
      <PriceInDollars price={price} percentOff={percentOff} />,
    );
    expect(container).toMatchSnapshot();
  });

  it("renders the price and percent off", () => {
    const price = 100;
    const percentOff = 0.2;
    const { container } = render(
      <PriceInDollars price={price} percentOff={percentOff} />,
    );
    expect(container).toMatchSnapshot();
  });
});
