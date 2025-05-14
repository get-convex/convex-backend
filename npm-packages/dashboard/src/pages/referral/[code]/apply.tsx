import Head from "next/head";
import { useTeamMembers, useTeams } from "api/teams";
import router from "next/router";
import { LoginLayout } from "layouts/LoginLayout";
import { useTeamOrbSubscription } from "api/billing";
import { RedeemReferralForm } from "components/referral/RedeemReferralForm";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useParams, usePathname } from "next/navigation";
import { useState } from "react";
import { Team } from "generatedApi";
import { useApplyReferralCode, useReferralCode } from "api/referrals";
import { useProfile } from "api/profile";
import { logEvent } from "convex-analytics";

/**
 *  This page powers two routes via Next.js rewrites in next.config.js:
 *  - /referral/THOMAS898/apply
 *  - /try-chef/THOMAS898/apply
 * */

export { getServerSideProps } from "lib/ssr";

function RedeemReferralCodePage() {
  const isChef = usePathname().includes("try-chef");

  const { code } = useParams<{ code: string }>();

  const { selectedTeamSlug: defaultTeamSlug, teams } = useTeams();
  const defaultTeam = teams?.find((t) => t.slug === defaultTeamSlug) ?? null;
  const [selectedTeam, setSelectedTeam] = useState<Team | null>(defaultTeam);
  const [isTeamSelectorShown, setIsTeamSelectorShown] = useState(false);

  const referralCode = useReferralCode(code);
  const teamEligibility = useTeamReedeemEligibility(selectedTeam);
  const applyReferralCode = useApplyReferralCode(selectedTeam?.id);

  return (
    <div className="h-screen">
      <Head>
        <title>Redeem your referral code | Convex Dashboard</title>
      </Head>
      <LoginLayout>
        <RedeemReferralForm
          referralCode={referralCode}
          teams={teams}
          selectedTeam={selectedTeam}
          onTeamSelect={setSelectedTeam}
          onShowTeamSelector={() => setIsTeamSelectorShown(true)}
          isTeamSelectorShown={isTeamSelectorShown}
          onSubmit={async () => {
            if (!selectedTeam) {
              throw new Error("No team selected");
            }

            await applyReferralCode({
              referralCode: code,
            });

            logEvent("redeemed referral code");

            if (isChef) {
              window.location.href = "https://chef.convex.dev";
            } else {
              void router.push(`/t/${selectedTeam.slug}`);
            }
          }}
          teamEligibility={teamEligibility}
          isChef={isChef}
        />
      </LoginLayout>
    </div>
  );
}

function useTeamReedeemEligibility(team: Team | null) {
  const { subscription } = useTeamOrbSubscription(team?.id);
  const isAdmin = useIsAdminOfTeam(team);

  if (team?.referredBy) {
    return { eligible: false, reason: "already_redeemed" } as const;
  }

  if (isAdmin === false) {
    return { eligible: false, reason: "not_admin" } as const;
  }

  if (subscription !== undefined && subscription !== null) {
    return { eligible: false, reason: "paid_subscription" } as const;
  }

  if (isAdmin === undefined || subscription === undefined) {
    return undefined;
  }

  return { eligible: true } as const;
}

function useIsAdminOfTeam(team: Team | null): boolean | undefined {
  const profile = useProfile();
  const members = useTeamMembers(team?.id);
  const member = members?.find((m) => m.id === profile?.id);
  return member === undefined ? undefined : member.role === "admin";
}

export default withAuthenticatedPage(RedeemReferralCodePage);
