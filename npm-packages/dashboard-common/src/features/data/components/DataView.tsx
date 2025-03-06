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
import { Modal } from "@common/elements/Modal";
import { LoadingTransition } from "@common/elements/Loading";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";

export function DataView() {
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
                      key={tableMetadata.name}
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
