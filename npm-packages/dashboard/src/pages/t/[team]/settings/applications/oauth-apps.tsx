import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import React from "react";
import { ApplicationsLayout } from "components/teamSettings/ApplicationsLayout";

export { getServerSideProps } from "lib/ssr";

function OauthApps() {
  return (
    <TeamSettingsLayout
      page="applications"
      Component={ApplicationsLayout}
      title="Applications"
    />
  );
}

export default withAuthenticatedPage(OauthApps);
