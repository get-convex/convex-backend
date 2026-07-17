import React, { useContext, useMemo } from "react";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { SidebarDetailLayout } from "@common/layouts/SidebarDetailLayout";
import { EmptyData } from "@common/features/data/components/EmptyData";
import {
  DataContent,
  DataContentSkeleton,
} from "@common/features/data/components/DataContent";
import {
  DataSidebar,
  DataSideBarSkeleton,
} from "@common/features/data/components/DataSidebar";
import {
  DeploymentInfoContext,
  PermissionsContext,
} from "@common/lib/deploymentContext";
import { useTableMetadataAndUpdateURL } from "@common/lib/useTableMetadata";
import { useNents } from "@common/lib/useNents";
import { SchemaJson } from "@common/lib/format";
import { LoadingTransition } from "@ui/Loading";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
import { useDataPageSize } from "./Table/utils/useQueryFilteredTable";

export function DataView({
  onTableCreated,
  onDocumentsAdded,
}: {
  onTableCreated?: () => void;
  onDocumentsAdded?: (count: number) => void;
}) {
  const { useCurrentDeployment, ErrorBoundary } = useContext(
    DeploymentInfoContext,
  );
  const { useIsOperationAllowed } = useContext(PermissionsContext);
  const deployment = useCurrentDeployment() ?? {
    id: undefined,
    kind: undefined,
  };

  const deploymentId = deployment && "id" in deployment ? deployment.id : null;

  const tableMetadata = useTableMetadataAndUpdateURL();

  const canViewData = useIsOperationAllowed("ViewData");

  const componentId = useNents().selectedNent?.id;
  const schemas = useQuery(
    udfs.getSchemas.default,
    canViewData ? { componentId: componentId ?? null } : "skip",
  );

  const [currentPageSize, setPageSize] = useDataPageSize(
    componentId ?? null,
    tableMetadata?.name ?? "",
  );

  const { activeSchema } = useMemo(() => {
    if (!schemas) {
      return { activeSchema: undefined };
    }

    return {
      activeSchema: schemas.active
        ? (JSON.parse(schemas.active) as SchemaJson)
        : null,
    };
  }, [schemas]);

  if (!canViewData) {
    return (
      <>
        <DeploymentPageTitle title="Data" />
        <NoPermissionMessage
          message="You do not have permission to view data in this deployment."
          missingPermission="deployment:data:view"
        />
      </>
    );
  }

  return (
    <>
      <DeploymentPageTitle
        subtitle={tableMetadata?.name ? "Data" : undefined}
        title={tableMetadata?.name || "Data"}
      />
      <LoadingTransition
        loadingProps={{ shimmer: false }}
        loadingState={
          <div className="size-full">
            <div className="flex h-full">
              <SidebarDetailLayout
                resizeHandleTitle="Tables"
                panelSizeKey={`${deploymentId}/data`}
                sidebarComponent={<DataSideBarSkeleton />}
                contentComponent={<DataContentSkeleton />}
              />
            </div>
          </div>
        }
      >
        {tableMetadata !== undefined && (
          <SidebarDetailLayout
            panelSizeKey={`${deploymentId}/data`}
            sidebarComponent={
              <DataSidebar
                tableData={tableMetadata}
                onTableCreated={onTableCreated}
              />
            }
            resizeHandleTitle="Tables"
            contentComponent={
              tableMetadata.name === null ? (
                <EmptyData />
              ) : (
                <LoadingTransition
                  loadingState={<DataContentSkeleton />}
                  loadingProps={{ shimmer: false }}
                >
                  {activeSchema !== undefined && (
                    <ErrorBoundary
                      fallback={(props) => (
                        <HandleTimeout
                          {...props}
                          setPageSize={setPageSize}
                          currentPageSize={currentPageSize}
                        />
                      )}
                    >
                      <DataContent
                        key={tableMetadata.name}
                        tableName={tableMetadata.name}
                        componentId={componentId ?? null}
                        activeSchema={activeSchema}
                        onDocumentsAdded={onDocumentsAdded}
                      />
                    </ErrorBoundary>
                  )}
                </LoadingTransition>
              )
            }
          />
        )}
      </LoadingTransition>
    </>
  );
}

function HandleTimeout({
  error,
  resetError,
  setPageSize,
  currentPageSize,
}: {
  error: Error;
  resetError(): void;
  currentPageSize: number;
  setPageSize: (pageSize: number) => void;
}) {
  if (
    error.message.startsWith(
      "[CONVEX Q(_system/frontend/paginatedTableDocuments:default)]",
    ) &&
    error.message.includes("Function execution timed out") &&
    currentPageSize !== 1
  ) {
    setPageSize(Math.floor(Math.max(currentPageSize / 2, 1)));
    resetError();
  } else {
    throw error;
  }
  return null;
}
