import {
  render,
  screen,
  fireEvent,
  act,
  getByText,
  getByRole,
} from "@testing-library/react";
import { OrbSubscriptionResponse, Team } from "generatedApi";
import { StarterPlan } from "./StarterPlan";

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
};

const team: Team = {
  id: 0,
  name: "",
  creator: 0,
  slug: "",
  suspended: false,
};

describe("StarterPlan", () => {
  beforeEach(() => {
    jest.resetAllMocks();
  });

  test("Downgrade plan button should not be visible if there is no subscription", () => {
    render(
      <StarterPlan subscription={undefined} hasAdminPermissions team={team} />,
    );

    const downgradeButton = screen.queryByText("Downgrade");
    expect(downgradeButton).not.toBeInTheDocument();

    screen.getByText("Current Plan");
  });

  test("Should be able to downgrade plan", async () => {
    const hasAdminPermissions = true;

    render(
      <StarterPlan
        subscription={subscription}
        hasAdminPermissions={hasAdminPermissions}
        team={team}
      />,
    );

    const downgradeButton = screen.getByText("Downgrade");
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

  test("Should say Current Plan if the plan is already Starter", () => {
    render(<StarterPlan hasAdminPermissions team={team} />);

    screen.getByText("Current Plan");
  });

  test("Should not be able to downgrade plan as non-admin", () => {
    const hasAdminPermissions = false;

    render(
      <StarterPlan
        subscription={subscription}
        hasAdminPermissions={hasAdminPermissions}
        team={team}
      />,
    );

    const downgradeButton = screen.getByText("Downgrade");
    expect(downgradeButton).toBeDisabled();
  });

  test("Should not be able to downgrade plan as admin with end date", () => {
    const hasAdminPermissions = true;

    render(
      <StarterPlan
        subscription={{ ...subscription, endDate: 0 }}
        hasAdminPermissions={hasAdminPermissions}
        team={team}
      />,
    );

    const downgradeButton = screen.queryByText("Downgrade");
    expect(downgradeButton).toBeNull();

    screen.getByText("Next Billing Cycle");
  });
});
