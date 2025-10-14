import { ProgressBar } from "@ui/ProgressBar";
import { ReferralState } from "generatedApi";

interface ReferralProgressProps {
  referralState: ReferralState;
}

export function ReferralProgress({ referralState }: ReferralProgressProps) {
  const referralsCount = referralState.referrals.length;
  const referralsComplete = referralsCount >= 5;

  return (
    <div className="hidden flex-col gap-1 md:flex">
      {!referralsComplete ? (
        <>
          <ProgressBar
            fraction={referralsCount / 5}
            ariaLabel="Referral progress"
            variant="solid"
            className="w-full"
          />
          <span className="text-sm font-medium whitespace-nowrap">
            {referralsCount}/5 referral boosts applied
          </span>
        </>
      ) : (
        <p className="max-w-[24ch] text-right text-sm font-medium text-balance xl:max-w-none">
          🎉 Congrats, your app limits have been boosted 5 times!
        </p>
      )}
    </div>
  );
}
