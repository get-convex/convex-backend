import React, { useContext, useEffect, useMemo, useState } from "react";
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
import { useRouter } from "next/router";
import omit from "lodash/omit";
import { useDataPageSize } from "./Table/utils/useQueryFilteredTable";

export function DataView() {
  const { useCurrentDeployment, ErrorBoundary } = useContext(
    DeploymentInfoContext,
  );
  const { id: deploymentId } = useCurrentDeployment() ?? {
    id: undefined,
    kind: undefined,
  };
  const router = useRouter();
  const tableMetadata = useTableMetadataAndUpdateURL();

  const componentId = useNents().selectedNent?.id;
  const schemas = useQuery(udfs.getSchemas.default, {
    componentId: componentId ?? null,
  });

  const [currentPageSize, setPageSize] = useDataPageSize(
    componentId ?? null,
    tableMetadata?.name ?? "",
  );

  const schemaValidationProgress = useQuery(
    udfs.getSchemas.schemaValidationProgress,
    {
      componentId: useNents().selectedNent?.id ?? null,
    },
  );

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

  useEffect(() => {
    if (router.query.showSchema === "true") {
      setIsShowingSchema(true);
      void router.push(
        {
          pathname: router.pathname,
          query: omit(router.query, "showSchema"),
        },
        undefined,
        { shallow: true },
      );
    }
  }, [router.query.showSchema, router]);

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
            schemaValidationProgress={schemaValidationProgress}
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
