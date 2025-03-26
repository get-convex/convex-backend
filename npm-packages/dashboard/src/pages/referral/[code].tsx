import Head from "next/head";
import { useTeamMembers, useTeams } from "api/teams";
import router from "next/router";
import { LoginLayout } from "layouts/LoginLayout";
import { useTeamOrbSubscription } from "api/billing";
import { RedeemReferralForm } from "components/referral/RedeemReferralForm";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useParams } from "next/navigation";
import { useState } from "react";
import { Team } from "generatedApi";
import { useApplyReferralCode } from "api/referrals";
import { useProfile } from "api/profile";

export { getServerSideProps } from "lib/ssr";

function RedeemReferralCodePage() {
  const { code } = useParams<{ code: string }>();

  const { selectedTeamSlug: defaultTeamSlug, teams } = useTeams();
  const defaultTeam = teams?.find((t) => t.slug === defaultTeamSlug) ?? null;
  const [selectedTeam, setSelectedTeam] = useState<Team | null>(defaultTeam);
  const [isTeamSelectorShown, setIsTeamSelectorShown] = useState(false);

  const teamEligibility = useTeamReedeemEligibility(selectedTeam);

  const applyReferralCode = useApplyReferralCode(selectedTeam?.id);

  const title = "Someone thinks you’re a good fit for Convex!";
  const description = "Get Convex resources for free with this referral code.";
  const ogImage = "https://www.convex.dev/og_image.png";

  return (
    <div className="h-screen">
      <Head>
        <title>{title}</title>
        <meta name="description" content={description} />

        <meta property="og:title" content={title} />
        <meta property="og:description" content={description} />

        <meta property="og:type" content="website" />
        <meta property="og:site_name" content="Convex" />
        <meta
          property="og:url"
          content={`https://dashboard.convex.dev/referral/${code}`}
        />
        <meta property="og:image" content={ogImage} />

        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:title" content={title} />
        <meta name="twitter:description" content={description} />
        <meta name="twitter:image" content={ogImage} />
      </Head>
      <LoginLayout>
        <RedeemReferralForm
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

            void router.push(`/t/${selectedTeam.slug}`);
          }}
          teamEligibility={teamEligibility}
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
