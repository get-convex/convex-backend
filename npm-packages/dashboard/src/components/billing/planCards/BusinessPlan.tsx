import { Button } from "@ui/Button";
import { OrbSubscriptionResponse } from "generatedApi";
import { useSupportFormOpen } from "elements/SupportWidget";
import { PlanCard } from "./PlanCard";

export function BusinessPlan({
  subscription,
}: {
  subscription?: OrbSubscriptionResponse;
}) {
  const isCurrentPlan = subscription?.plan.planType === "CONVEX_BUSINESS";
  const [, setOpenState] = useSupportFormOpen();

  return (
    <PlanCard
      selected={isCurrentPlan}
      plan={{
        id: "CONVEX_BUSINESS",
        planType: "CONVEX_BUSINESS",
        name: "Business",
        description: "For teams that need custom limits and priority support.",
        status: "active",
        seatPrice: null,
      }}
      saleHeader="$2,500 monthly minimum"
      action={
        isCurrentPlan ? (
          <p className="flex h-[2.125rem] items-center px-2 font-semibold">
            Current Plan
          </p>
        ) : (
          <Button
            variant="neutral"
            onClick={() =>
              setOpenState({
                defaultSubject: "Business Plan Inquiry",
                defaultMessage:
                  "I'm interested in learning more about the Business plan.",
              })
            }
          >
            Contact Us
          </Button>
        )
      }
    />
  );
}
