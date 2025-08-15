import { Referrals } from "components/referral/Referrals";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function ReferralsPage() {
  return (
    <TeamSettingsLayout
      page="referrals"
      Component={Referrals}
      title="Referrals"
    />
  );
}

export default withAuthenticatedPage(ReferralsPage);
