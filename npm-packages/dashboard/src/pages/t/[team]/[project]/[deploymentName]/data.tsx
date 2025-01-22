import React, { useMemo, useState } from "react";
import { SidebarDetailLayout } from "layouts/SidebarDetailLayout";
import {
  useTableMetadataAndUpdateURL,
  LoadingTransition,
  useNents,
  useTableShapes,
  SchemaJson,
} from "dashboard-common";
import { EmptyData } from "components/dataBrowser/EmptyData";
import {
  DataContent,
  DataContentSkeleton,
} from "components/dataBrowser/DataContent";
import {
  DataSidebar,
  DataSideBarSkeleton,
} from "components/dataBrowser/DataSidebar";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ShowSchema } from "components/dataBrowser/ShowSchema";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { Modal } from "elements/Modal";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { useCurrentDeployment } from "api/deployments";

export { getServerSideProps } from "lib/ssr";

function DataView() {
  const deploymentId = useCurrentDeployment()?.id;
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

  // Still loading the tables.
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
            contentComponent={
              tableMetadata.name === null ? (
                <EmptyData />
              ) : (
                <LoadingTransition
                  loadingState={<DataContentSkeleton />}
                  loadingProps={{ shimmer: false }}
                >
                  {activeSchema !== undefined && (
                    <DataContent
                      tableName={tableMetadata.name}
                      componentId={componentId ?? null}
                      shape={
                        tableMetadata.tables.get(tableMetadata.name) ?? null
                      }
                      activeSchema={activeSchema}
                    />
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

export default withAuthenticatedPage(DataView);
