import React, { useContext, useMemo, useState } from "react";
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
import { ShowSchema } from "@common/features/data/components/ShowSchema";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useTableMetadataAndUpdateURL } from "@common/lib/useTableMetadata";
import { useNents } from "@common/lib/useNents";
import { SchemaJson } from "@common/lib/format";
import { useTableShapes } from "@common/lib/deploymentApi";
import { Modal } from "@ui/Modal";
import { LoadingTransition } from "@ui/Loading";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { Callout } from "@ui/Callout";

export function DataView() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const { id: deploymentId, kind } = useCurrentDeployment() ?? {
    id: undefined,
    kind: undefined,
  };
  const tableMetadata = useTableMetadataAndUpdateURL();

  const componentId = useNents().selectedNent?.id;
  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: componentId ?? null,
  });

  const { activeSchema, inProgressSchema } = useMemo(() => {
    if (!schemas) return {};

    return {
      activeSchema: schemas.active
        ? (JSON.parse(schemas.active) as SchemaJson)
        : null,
      inProgressSchema: schemas.inProgress
        ? (JSON.parse(schemas.inProgress) as SchemaJson)
        : null,
    };
  }, [schemas]);

  const { tables, hadError } = useTableShapes();
  const { ErrorBoundary } = useContext(DeploymentInfoContext);

  const [isShowingSchema, setIsShowingSchema] = useState(false);
  const showSchemaProps = useMemo(
    () =>
      activeSchema === undefined || inProgressSchema === undefined
        ? undefined
        : {
            hasSaved: activeSchema !== null || inProgressSchema !== null,
            showSchema: () => setIsShowingSchema(true),
          },
    [activeSchema, inProgressSchema, setIsShowingSchema],
  );

  return (
    <>
      <DeploymentPageTitle
        subtitle={tableMetadata?.name ? "Data" : undefined}
        title={tableMetadata?.name || "Data"}
      />
      {schemas && tables && isShowingSchema && (
        <Modal
          onClose={() => setIsShowingSchema(false)}
          title={<div className="px-3">Schema</div>}
          size="md"
        >
          <ShowSchema
            activeSchema={activeSchema}
            inProgressSchema={inProgressSchema}
            shapes={tables}
            hasShapeError={hadError}
          />
        </Modal>
      )}
      <LoadingTransition
        loadingProps={{ shimmer: false }}
        loadingState={
          <div className="h-full w-full">
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
                showSchema={showSchemaProps}
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
                      fallback={({ error }) => {
                        // Old versions of local deployments don't have the frontend/indexes.js system UDF that now reads indexes.
                        // We force folks to upgrade their local deployment to avoid supporting multiple codepaths for reading indexes.
                        if (
                          error.message.includes(
                            "Couldn't find system module '\"frontend/indexes.js\"",
                          ) &&
                          kind === "local"
                        ) {
                          return (
                            <div className="h-full grow">
                              <div className="flex h-full flex-col items-center justify-center">
                                <Callout
                                  variant="error"
                                  className="max-w-prose"
                                >
                                  <span>
                                    Your local deployment is out of date. Please
                                    restart it with <code>npx convex dev</code>{" "}
                                    and upgrade.
                                  </span>
                                </Callout>
                              </div>
                            </div>
                          );
                        }
                        throw error;
                      }}
                    >
                      <DataContent
                        key={tableMetadata.name}
                        tableName={tableMetadata.name}
                        componentId={componentId ?? null}
                        shape={
                          tableMetadata.tables.get(tableMetadata.name) ?? null
                        }
                        activeSchema={activeSchema}
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
