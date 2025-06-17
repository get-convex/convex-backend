import {
  render,
  screen,
  fireEvent,
  act,
  getByText,
  getByRole,
} from "@testing-library/react";
import { OrbSubscriptionResponse, Team } from "generatedApi";
import { FreePlan } from "./FreePlan";

const cancelSubscription = jest.fn();

jest.mock("api/billing", () => ({
  useCancelSubscription: () => cancelSubscription,
}));

const setSupportFormOpen = jest.fn();
jest.mock("../../../elements/SupportWidget", () => ({
  useSupportFormOpen: () => [false, setSupportFormOpen],
}));

const subscription: OrbSubscriptionResponse = {
  plan: {
    id: "",
    name: "",
    description: "",
    status: "active",
    seatPrice: 0,
    planType: "",
  },
  billingContact: {
    name: "",
    email: "",
  },
  status: "active",
  nextBillingPeriodStart: "2025-09-25",
};

const team: Team = {
  id: 0,
  name: "",
  creator: 0,
  slug: "",
  suspended: false,
  referralCode: "CODE123",
};

describe("FreePlan", () => {
  beforeEach(() => {
    jest.resetAllMocks();
  });

  test("Downgrade plan button should not be visible if there is no subscription", () => {
    render(
      <FreePlan subscription={undefined} hasAdminPermissions team={team} />,
    );

    const downgradeButton = screen.queryByText("Downgrade to Free");
    expect(downgradeButton).not.toBeInTheDocument();

    screen.getByText("Current Plan");
  });

  test("Should be able to downgrade plan", async () => {
    const hasAdminPermissions = true;

    render(
      <FreePlan
        subscription={subscription}
        hasAdminPermissions={hasAdminPermissions}
        team={team}
      />,
    );

    const downgradeButton = screen.getByText("Downgrade to Free");
    await act(() => {
      fireEvent.click(downgradeButton);
    });

    const confirmationDialog = screen.getByRole("dialog");
    expect(confirmationDialog).toBeInTheDocument();

    const confirmButton = getByText(confirmationDialog, "Downgrade");
    expect(confirmButton).toBeDisabled();

    const checkbox = getByRole(confirmationDialog, "checkbox");

    await act(() => {
      checkbox.click();
    });

    expect(confirmButton).toBeEnabled();

    expect(cancelSubscription).toHaveBeenCalledTimes(0);

    await act(() => {
      confirmButton.click();
    });

    expect(cancelSubscription).toHaveBeenCalledTimes(1);
  });

  test("Should say Current Plan if the plan is already Free", () => {
    render(<FreePlan hasAdminPermissions team={team} />);

    screen.getByText("Current Plan");
  });

  test("Should not be able to downgrade plan as non-admin", () => {
    const hasAdminPermissions = false;

    render(
      <FreePlan
        subscription={subscription}
        hasAdminPermissions={hasAdminPermissions}
        team={team}
      />,
    );

    const downgradeButton = screen.getByText("Downgrade to Free");
    expect(downgradeButton).toBeDisabled();
  });

  test("Should not be able to downgrade plan as admin with end date", () => {
    const hasAdminPermissions = true;

    render(
      <FreePlan
        subscription={{ ...subscription, endDate: 0 }}
        hasAdminPermissions={hasAdminPermissions}
        team={team}
      />,
    );

    const downgradeButton = screen.queryByText("Downgrade to Free");
    expect(downgradeButton).toBeNull();

    screen.getByText("Next Billing Cycle");
  });
});
