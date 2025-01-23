import { TeamMembers } from "components/teamSettings/TeamMembers";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function TeamSettingsPage() {
  return (
    <TeamSettingsLayout
      page="members"
      Component={TeamMembers}
      title="Members"
    />
  );
}

export default withAuthenticatedPage(TeamSettingsPage);
