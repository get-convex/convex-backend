import { CheckIcon, CopyIcon, DotsVerticalIcon } from "@radix-ui/react-icons";
import { logEvent } from "convex-analytics";
import { Menu, MenuItem } from "dashboard-common/elements/Menu";
import { ProgressBar } from "dashboard-common/elements/ProgressBar";
import { TextInput } from "dashboard-common/elements/TextInput";
import { cn } from "dashboard-common/lib/cn";
import { useCopy } from "dashboard-common/lib/useCopy";
import { ReferralState, Team } from "generatedApi";
import { useId, useState } from "react";

interface ReferralsBannerProps {
  team: Team;
  referralState: ReferralState;
  onHide?: () => void;
  className?: string;
}

export function ReferralsBanner({
  team,
  referralState,
  onHide,
  className,
}: ReferralsBannerProps) {
  const [copied, setCopied] = useState(false);
  const id = useId();

  const referralsCount = referralState?.referrals.length || 0;
  const referralsComplete = referralsCount >= 5;
  const referralCode = team?.referralCode;

  const copy = useCopy("Referral link");

  return (
    <div
      className={cn(
        "border border-purple-400 rounded-md flex bg-background-secondary",
        className,
      )}
    >
      <div className="flex grow gap-2 py-2 pl-4 md:items-center md:justify-between">
        <div className="flex grow flex-col gap-2 xl:flex-row xl:items-center">
          <span className="text-balance text-sm">
            Boost your account limits up to 5× by sharing your referral code:{" "}
          </span>
          <div className="w-72">
            <TextInput
              id={id}
              labelHidden
              value={`convex.dev/referral/${referralCode}`}
              readOnly
              Icon={copied ? CheckIcon : CopyIcon}
              iconTooltip={copied ? "Copied" : "Copy to clipboard"}
              action={async () => {
                copy(`https://www.convex.dev/referral/${referralCode}`);
                logEvent("copied referral link from referrals banner");

                setCopied(true);
                setTimeout(() => {
                  setCopied(false);
                }, 3000);
              }}
            />
          </div>
        </div>
        <div className="hidden flex-col gap-1 md:flex xl:grow xl:flex-row-reverse xl:items-center xl:gap-2">
          {!referralsComplete ? (
            <>
              <ProgressBar
                fraction={referralsCount / 5}
                ariaLabel="Referral progress"
                variant="solid"
                className="w-full"
              />
              <span className="whitespace-nowrap text-sm font-medium">
                {referralsCount}/5 referral boosts applied
              </span>
            </>
          ) : (
            <p className="max-w-[24ch] text-balance text-right text-sm font-medium xl:max-w-none">
              🎉 Congrats, your app limits have been boosted 5 times!
            </p>
          )}
        </div>
      </div>
      <Menu
        placement="bottom-start"
        buttonProps={{
          "aria-label": "Open project settings",
          variant: "neutral",
          inline: true,
          icon: <DotsVerticalIcon className="text-content-secondary" />,
        }}
      >
        <MenuItem action={onHide}>Hide banner</MenuItem>
        <MenuItem href={`/t/${team?.slug}/settings/referrals`}>
          View referrals
        </MenuItem>
      </Menu>
    </div>
  );
}
