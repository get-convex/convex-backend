import React from "react";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { FunctionsView } from "dashboard-common";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function FunctionsPage() {
  return (
    <>
      <DeploymentPageTitle title="Functions" />
      <FunctionsView />
    </>
  );
});
