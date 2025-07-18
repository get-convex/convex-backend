import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import React from "react";
import { ApplicationsLayout } from "components/teamSettings/ApplicationsLayout";

export { getServerSideProps } from "lib/ssr";

function Applications() {
  return (
    <TeamSettingsLayout
      page="applications"
      Component={ApplicationsLayout}
      title="Applications"
    />
  );
}

export default withAuthenticatedPage(Applications);
