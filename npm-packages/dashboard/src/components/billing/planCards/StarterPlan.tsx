import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { formatDate } from "@common/lib/format";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Checkbox } from "@ui/Checkbox";
import { useState } from "react";
import Link from "next/link";
import { useCancelSubscription } from "api/billing";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { OrbSubscriptionResponse, Team } from "generatedApi";
import { PlanCard } from "./PlanCard";

export function StarterPlan({
  subscription,
  hasAdminPermissions,
  team,
}: {
  subscription?: OrbSubscriptionResponse;
  hasAdminPermissions: boolean;
  team: Team;
}) {
  const [isSelfServeDowngradeModalOpen, setIsSelfServeDowngradeModalOpen] =
    useState(false);
  const [acceptedConsequences, setAcceptedConsequences] = useState(false);
  const cancelSubscription = useCancelSubscription(team.id);
  return (
    <>
      <PlanCard
        selected={!subscription}
        plan={{
          id: "CONVEX_BASE",
          planType: "CONVEX_BASE",
          name: "Starter",
          description: "For hobbyists and prototypes.",
          status: "active",
          seatPrice: 0,
        }}
        saleHeader="Free forever"
        action={
          !subscription ? (
            <p className="flex h-[2.125rem] items-center font-semibold">
              Current Plan
            </p>
          ) : typeof subscription.endDate === "number" ? (
            <p className="flex items-center gap-1 py-2 font-semibold">
              Next Billing Cycle{" "}
              <Tooltip
                tip={`Your subscription has been canceled and will end on ${formatDate(new Date(subscription.endDate))}. You may resume the subscription before then to avoid losing access to features.`}
              >
                <InfoCircledIcon />
              </Tooltip>
            </p>
          ) : (
            <Button
              disabled={!hasAdminPermissions}
              tip={
                !hasAdminPermissions
                  ? "You do not have permission to modify the team subscription."
                  : typeof subscription.endDate === "number"
                    ? `Your subscription has already been canceled and will end on ${formatDate(new Date(subscription.endDate))}. You may resume the subscription before then to avoid losing access to features.`
                    : undefined
              }
              variant="neutral"
              onClick={() => {
                setIsSelfServeDowngradeModalOpen(true);
              }}
            >
              Downgrade to Starter
            </Button>
          )
        }
      />
      {isSelfServeDowngradeModalOpen && (
        <ConfirmationDialog
          onClose={() => setIsSelfServeDowngradeModalOpen(false)}
          dialogTitle="Downgrade to Convex Starter"
          dialogBody={
            <div className="flex flex-col gap-4">
              <p>
                Are you sure you want to downgrade to the Convex Starter plan?
              </p>
              <p>
                Once you downgrade, your team will lose access to all features
                that are not included in the Starter plan at the end of the
                current billing period.
              </p>
              <p>
                If this team's{" "}
                <Link
                  className="text-content-link hover:underline"
                  href={`/t/${team?.slug}/settings/usage`}
                >
                  usage
                </Link>{" "}
                exceeds the Starter plan limits, your projects may be
                automatically disabled.
              </p>
              <label className="mx-1 flex gap-2 text-sm">
                <Checkbox
                  className="mt-0.5"
                  checked={acceptedConsequences}
                  onChange={(e) =>
                    setAcceptedConsequences(e.currentTarget.checked)
                  }
                />{" "}
                By checking this box, I acknowledge my team may lose access to
                features or projects exceeding Starter plan usage limits.
              </label>
            </div>
          }
          variant="danger"
          confirmText="Downgrade"
          disableConfirm={!acceptedConsequences}
          onConfirm={async () => {
            await cancelSubscription();
          }}
        />
      )}
    </>
  );
}
