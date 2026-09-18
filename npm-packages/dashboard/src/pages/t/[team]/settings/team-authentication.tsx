import { TeamSSO } from "components/teamSettings/teamAuthentication/TeamSSO";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function TeamAuthenticationPage() {
  return (
    <TeamSettingsLayout
      page="team-authentication"
      Component={TeamSSO}
      title="Team Authentication"
    />
  );
}

export default withAuthenticatedPage(TeamAuthenticationPage);
