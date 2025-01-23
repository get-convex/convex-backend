import { act, fireEvent, render, screen } from "@testing-library/react";
import { formatDate } from "dashboard-common";
import { OrbSubscriptionResponse, Team } from "generatedApi";
import { SubscriptionOverview } from "./SubscriptionOverview";

const resumeSubscription = jest.fn();
jest.mock("api/billing", () => ({
  useResumeSubscription: () => resumeSubscription,
  useListInvoices: () => [],
  useUpdateBillingContact: () => jest.fn(),
  useUpdateBillingAddress: () => jest.fn(),
}));

jest.mock("../../hooks/useStripe", () => ({
  useStripeAddressSetup: jest
    .fn()
    .mockReturnValue({ options: { clientSecret: undefined } }),
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

describe("SubscriptionOverview", () => {
  test("can resume canceled subscription", async () => {
    const hasAdminPermissions = true;
    const endDate = 0;

    render(
      <SubscriptionOverview
        team={team}
        hasAdminPermissions={hasAdminPermissions}
        subscription={{ ...subscription, endDate }}
      />,
    );

    expect(resumeSubscription).not.toHaveBeenCalled();

    screen.getByText("Subscription ends on");
    // The date is an a separate element
    screen.getByText(formatDate(new Date(endDate)));
    await act(async () => {
      fireEvent.click(screen.getByText("Resume Subscription"));
    });

    expect(resumeSubscription).toHaveBeenCalled();
  });

  test("cannot resume active subscription", () => {
    const hasAdminPermissions = true;

    render(
      <SubscriptionOverview
        team={team}
        hasAdminPermissions={hasAdminPermissions}
        subscription={{ ...subscription, endDate: null }}
      />,
    );

    expect(screen.queryByText("Subscription ends on")).toBeNull();
    expect(screen.queryByText("Resume Subscription")).toBeNull();
  });
});
