import { Checkbox } from "@ui/Checkbox";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { useTeamMembers } from "api/teams";
import { PriceSummary } from "components/billing/PriceSummary";
import { PlanResponse, Team } from "generatedApi";
import Link from "next/link";
import { useState } from "react";

export function DowngradePlanDialog({
  onClose,
  onConfirm,
  newPlan,
  team,
}: {
  onClose: () => void;
  onConfirm: () => Promise<void>;
  newPlan: PlanResponse;
  team: Team;
}) {
  const [acceptedConsequences, setAcceptedConsequences] = useState(false);
  return (
    <ConfirmationDialog
      onClose={onClose}
      dialogTitle={`Downgrade to ${newPlan.name}`}
      dialogBody={
        <div className="flex flex-col gap-4">
          <p>Are you sure you want to downgrade to the {newPlan.name} plan?</p>
          <p>
            Once you downgrade, your team will lose access to all features that
            are not included in the {newPlan.name} plan{" "}
            {newPlan.planType === "CONVEX_STARTER"
              ? "at the end of the current billing period."
              : "immediately."}
          </p>
          {newPlan.planType === "CONVEX_STARTER" && (
            <p>
              If this team's{" "}
              <Link
                className="text-content-link hover:underline"
                href={`/t/${team?.slug}/settings/usage`}
              >
                usage
              </Link>{" "}
              exceeds the {newPlan.name} plan limits, your projects may be
              automatically disabled.
            </p>
          )}
          <label className="mx-1 flex gap-2 text-sm">
            <Checkbox
              className="mt-0.5"
              checked={acceptedConsequences}
              onChange={(e) => setAcceptedConsequences(e.currentTarget.checked)}
            />{" "}
            By checking this box, I acknowledge my team may lose access to
            features
            {newPlan.planType === "CONVEX_STARTER" &&
              ` and projects exceeding ${newPlan.name} plan usage limits`}
            .
          </label>
        </div>
      }
      variant="danger"
      confirmText="Downgrade"
      disableConfirm={!acceptedConsequences}
      onConfirm={onConfirm}
    />
  );
}

export function UpgradePlanDialog({
  onClose,
  onConfirm,
  newPlan,
  team,
}: {
  onClose: () => void;
  onConfirm: () => Promise<void>;
  newPlan: PlanResponse;
  team: Team;
}) {
  const teamMembers = useTeamMembers(team.id);
  return (
    <ConfirmationDialog
      onClose={onClose}
      dialogTitle={`Upgrade to ${newPlan.name}`}
      dialogBody={
        <div>
          <PriceSummary
            plan={newPlan}
            teamMemberDiscountPct={0}
            numMembers={teamMembers?.length || 1}
            requiresPaymentMethod
            couponDurationInMonths={undefined}
            isUpgrading
          />
        </div>
      }
      variant="primary"
      confirmText="Upgrade"
      onConfirm={onConfirm}
    />
  );
}
