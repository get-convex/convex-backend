import { Button } from "dashboard-common/elements/Button";
import { useSupportFormOpen } from "elements/SupportWidget";
import { PlanCard } from "./PlanCard";

export function BusinessPlan({
  hasAdminPermissions,
}: {
  hasAdminPermissions: boolean;
}) {
  const [, setOpenState] = useSupportFormOpen();
  return (
    <PlanCard
      selected={false}
      plan={{
        id: "CONVEX_BUSINESS",
        planType: "CONVEX_BUSINESS",
        name: "Business",
        description:
          "For teams needing advanced features, instant support, and serious scale.",
        status: "active",
        seatPrice: 0,
      }}
      saleHeader="Let's talk"
      action={
        <Button
          tip={
            !hasAdminPermissions &&
            "You do not have permission to modify the team subscription."
          }
          onClick={() => {
            setOpenState({
              defaultSubject: "Upgrade to Convex Business",
              defaultMessage: `Please tell us a bit about the capabilities you're looking for:\n\n`,
            });
          }}
          disabled={!hasAdminPermissions}
        >
          Contact Us
        </Button>
      }
    />
  );
}
