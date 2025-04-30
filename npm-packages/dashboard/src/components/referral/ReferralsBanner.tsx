import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { Menu, MenuItem } from "@ui/Menu";
import { cn } from "@ui/cn";
import { ReferralState, Team } from "generatedApi";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { ReferralProgress } from "./ReferralProgress";

interface ReferralsBannerProps {
  team: Team;
  referralState: ReferralState;
  onHide: () => void;
  className?: string;
}

export function ReferralsBanner({
  team,
  referralState,
  onHide,
  className,
}: ReferralsBannerProps) {
  const referralCode = team?.referralCode;

  return (
    <div
      className={cn(
        "border rounded-md flex items-center gap-2 bg-background-secondary pl-4 pr-2 overflow-x-auto",
        className,
      )}
    >
      <div className="flex grow items-center gap-2 py-2 md:justify-between">
        <div className="flex max-w-prose grow flex-col flex-wrap gap-2 xl:flex-row xl:items-center">
          <span className="text-balance text-sm font-medium">
            Boost your resource usage limits by up to 5 times by sharing your
            referral code{" "}
          </span>
          <div className="w-72">
            <CopyTextButton
              text={`https://convex.dev/referral/${referralCode}`}
            />
          </div>
        </div>
        <div className="hidden flex-col gap-1 md:flex xl:grow xl:flex-row-reverse xl:items-center xl:gap-2">
          <ReferralProgress referralState={referralState} />
        </div>
      </div>
      <Menu
        placement="bottom-start"
        buttonProps={{
          "aria-label": "Open project settings",
          variant: "neutral",
          inline: true,
          className: "w-fit h-fit",
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
