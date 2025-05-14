import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RedeemReferralLanding } from "./RedeemReferralLanding";

jest.mock("convex-analytics", () => ({
  logEvent: jest.fn(),
}));

describe("RedeemReferralLanding", () => {
  // We’re applying a onClick event to the link. This test verifies it is still a <a> and not a
  // button to ensure there is no regression on the click behavior.
  it("has a link that becomes disabled when clicked", async () => {
    const code = "TEST123";
    render(
      <RedeemReferralLanding title="Test Title" code={code} isChef={false} />,
    );

    const link = screen.getByRole("link", { name: "Sign up with GitHub" });
    expect(link).toHaveAttribute(
      "href",
      `/api/auth/login?returnTo=%2Freferral%2F${code}%2Fapply`,
    );
    expect(link).toHaveAttribute("aria-disabled", "false");

    await act(async () => {
      await userEvent.click(link);
    });

    expect(jest.requireMock("convex-analytics").logEvent).toHaveBeenCalledWith(
      "clicked “Sign up with GitHub” through referral landing",
    );
    expect(link).toHaveAttribute("aria-disabled", "true");
  });
});
