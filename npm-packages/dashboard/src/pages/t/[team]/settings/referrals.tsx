import { Referrals } from "components/referral/Referrals";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useRouter } from "next/router";

export { getServerSideProps } from "lib/ssr";

function ReferralsPage() {
  const { referralsPage } = useLaunchDarkly();
  const router = useRouter();

  if (!referralsPage) {
    void router.push("/404");
    return null;
  }

  return (
    <TeamSettingsLayout
      page="referrals"
      Component={Referrals}
      title="Referrals"
    />
  );
}

export default withAuthenticatedPage(ReferralsPage);
