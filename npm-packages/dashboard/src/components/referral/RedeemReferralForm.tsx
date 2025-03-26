import { Button } from "dashboard-common/elements/Button";
import { Combobox } from "dashboard-common/elements/Combobox";
import { useState } from "react";
import { Spinner } from "dashboard-common/elements/Spinner";
import { Sheet } from "dashboard-common/elements/Sheet";
import { Callout } from "dashboard-common/elements/Callout";
import { Team } from "generatedApi";
import { Loading } from "dashboard-common/elements/Loading";
import { ReferralsBenefits } from "components/referral/ReferralsBenefits";

type TeamEligibilityError =
  | "paid_subscription"
  | "already_redeemed"
  | "not_admin";

export function RedeemReferralForm({
  teams,
  selectedTeam,
  onTeamSelect,
  onSubmit,
  isTeamSelectorShown,
  onShowTeamSelector,
  teamEligibility,
}: {
  teams: Team[] | undefined;
  selectedTeam: Team | null;
  onTeamSelect: (team: Team) => void;
  onSubmit: () => Promise<void>;
  isTeamSelectorShown: boolean;
  onShowTeamSelector: () => void;
  teamEligibility:
    | undefined
    | { eligible: true }
    | { eligible: false; reason: TeamEligibilityError };
}) {
  const [isSubmitting, setIsSubmitting] = useState(false);

  return (
    <div className="flex w-screen items-center justify-center">
      <Sheet className="flex w-full max-w-prose flex-col gap-2">
        <h3>Someone thinks you’re a good fit for Convex!</h3>

        <p className="text-content-primary">
          Thanks to your referral code, you will get the following resources for
          free on top of your free plan limits:
        </p>

        <ul className="mb-3 mt-4 grid gap-x-2 gap-y-4 sm:grid-cols-2">
          <ReferralsBenefits />
        </ul>

        {teams === undefined ? (
          <Loading fullHeight={false} className="h-28" />
        ) : (
          <form
            onSubmit={async (e) => {
              e.preventDefault();

              setIsSubmitting(true);
              try {
                await onSubmit();
              } finally {
                setIsSubmitting(false);
              }
            }}
            className="flex flex-col gap-4"
          >
            {isTeamSelectorShown ||
            selectedTeam === null ||
            teamEligibility?.eligible === false ? (
              <div className="flex flex-col gap-1">
                <Combobox
                  labelHidden={false}
                  options={teams.map((t) => ({
                    label: t.name,
                    value: t.slug,
                  }))}
                  label="Apply the code to team"
                  selectedOption={selectedTeam?.slug ?? null}
                  setSelectedOption={(slug) => {
                    if (slug !== null) {
                      const team = teams?.find((t) => t.slug === slug);
                      if (team) {
                        onTeamSelect(team);
                      }
                    }
                  }}
                  disableSearch
                />
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <p className="grow text-content-secondary">
                  These free resources will be added to the team{" "}
                  <strong className="font-semibold">{selectedTeam.name}</strong>
                  .{" "}
                </p>
                <Button
                  variant="neutral"
                  onClick={onShowTeamSelector}
                  size="xs"
                >
                  Change team
                </Button>
              </div>
            )}

            {teamEligibility?.eligible === false && (
              <Callout variant="error" className="my-0">
                {teamEligibilityErrorMessage(teamEligibility.reason)}
              </Callout>
            )}

            <div>
              <Button
                type="submit"
                disabled={
                  !selectedTeam ||
                  isSubmitting ||
                  teamEligibility === undefined ||
                  !teamEligibility.eligible
                }
              >
                {isSubmitting ? (
                  <Spinner className="h-4 w-4" />
                ) : (
                  <div className="flex items-center gap-2">
                    Get my free resources
                    {teamEligibility === undefined && (
                      <Spinner className="h-4 w-4" />
                    )}
                  </div>
                )}
              </Button>
            </div>
          </form>
        )}
      </Sheet>
    </div>
  );
}

function teamEligibilityErrorMessage(error: TeamEligibilityError) {
  switch (error) {
    case "paid_subscription":
      return "You cannot redeem a referral code for a team that has a paid subscription.";
    case "already_redeemed":
      return "This team has already redeemed a referral code.";
    case "not_admin":
      return "You must be an admin of the team to redeem a referral code.";
    default: {
      const exhaustiveCheck: never = error;
      throw new Error(`Unknown team eligibility error: ${exhaustiveCheck}`);
    }
  }
}
