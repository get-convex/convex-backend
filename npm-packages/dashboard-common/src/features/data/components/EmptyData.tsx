import { PlusIcon, TableIcon } from "@radix-ui/react-icons";
import { useContext } from "react";
import { CreateNewTable } from "./DataSidebar";
import { EmptySection } from "../../../elements/EmptySection";
import { useNents } from "../../../lib/useNents";
import { DeploymentInfoContext } from "../../../lib/deploymentContext";
import { useTableMetadata } from "../../../lib/useTableMetadata";
import { useLogDeploymentEvent } from "../../../lib/deploymentApi";
import { Loading } from "../../../elements/Loading";
import { Button } from "../../../elements/Button";

export function EmptyData() {
  return (
    <div className="flex h-full p-6">
      <EmptyDataContent noTables />
    </div>
  );
}

export function EmptyDataContent({
  noTables,
  openAddDocuments,
}: {
  noTables?: boolean;
  openAddDocuments?: () => void;
}) {
  const { selectedNent } = useNents();

  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );

  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canAddDocuments =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;
  const tableMetadata = useTableMetadata();
  const log = useLogDeploymentEvent();
  if (!tableMetadata) {
    return <Loading />;
  }

  return (
    <EmptySection
      Icon={TableIcon}
      header={
        noTables ? "There are no tables here yet." : "This table is empty."
      }
      sheet={!noTables}
      body={
        noTables
          ? "Create a table to start storing data."
          : "Create a document or run a mutation to start storing data."
      }
      action={
        noTables ? (
          <CreateNewTable tableData={tableMetadata} />
        ) : (
          <>
            {openAddDocuments && (
              <Button
                inline
                onClick={() => {
                  log("open add documents panel", { how: "empty data" });
                  openAddDocuments();
                }}
                size="sm"
                disabled={
                  !canAddDocuments ||
                  !!(selectedNent && selectedNent.state !== "active")
                }
                tip={
                  selectedNent && selectedNent.state !== "active"
                    ? "Cannot add documents in an unmounted component."
                    : !canAddDocuments &&
                      "You do not have permission to add documents in production."
                }
                icon={<PlusIcon aria-hidden="true" />}
              >
                Add Documents
              </Button>
            )}
          </>
        )
      }
      learnMoreButton={{
        href: "https://docs.convex.dev/quickstarts",
        children: "Follow a quickstart guide for your favorite framework.",
      }}
    />
  );
}
