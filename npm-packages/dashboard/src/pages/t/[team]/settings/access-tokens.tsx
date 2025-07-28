import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import React from "react";
import { TokensLayout } from "components/teamSettings/TokensLayout";

export { getServerSideProps } from "lib/ssr";

function Tokens() {
  return (
    <TeamSettingsLayout
      page="access-tokens"
      Component={TokensLayout}
      title="Access Tokens"
    />
  );
}

export default withAuthenticatedPage(Tokens);
