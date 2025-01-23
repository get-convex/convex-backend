import React, { useContext, useMemo, useState } from "react";
import {
  useTableMetadataAndUpdateURL,
  LoadingTransition,
  useNents,
  useTableShapes,
  SchemaJson,
  DeploymentInfoContext,
  Modal,
} from "dashboard-common";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { SidebarDetailLayout } from "../../../layouts/SidebarDetailLayout";
import { EmptyData } from "./EmptyData";
import { DataContent, DataContentSkeleton } from "./DataContent";
import { DataSidebar, DataSideBarSkeleton } from "./DataSidebar";
import { ShowSchema } from "./ShowSchema";

export function Data() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
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

  return (
    <>
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
