import { TeamSSO } from "components/teamSettings/TeamSSO";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function SSOPage() {
  return <TeamSettingsLayout page="sso" Component={TeamSSO} title="SSO" />;
}

export default withAuthenticatedPage(SSOPage);
