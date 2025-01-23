import React from "react";
import { Data, useTableMetadataAndUpdateURL } from "dashboard-common";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";

export { getServerSideProps } from "lib/ssr";

function DataView() {
  const tableMetadata = useTableMetadataAndUpdateURL();

  return (
    <>
      <DeploymentPageTitle
        subtitle={tableMetadata?.name ? "Data" : undefined}
        title={tableMetadata?.name || "Data"}
      />
      <Data />
    </>
  );
}

export default withAuthenticatedPage(DataView);
