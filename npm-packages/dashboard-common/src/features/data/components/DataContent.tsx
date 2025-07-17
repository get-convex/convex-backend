import { useRouter } from "next/router";
import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { FixedSizeList } from "react-window";

import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import classNames from "classnames";
import {
  Filter,
  FilterByIndex,
  FilterByIndexRange,
  SchemaJson,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Shape } from "shapes";
import { LoadingLogo, LoadingTransition } from "@ui/Loading";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useSelectionState } from "@common/features/data/lib/useSelectionState";
import { useDataToolbarActions } from "@common/features/data/lib/useDataToolbarActions";
import { useTableFilters } from "@common/features/data/lib/useTableFilters";
import { useToolPopup } from "@common/features/data/lib/useToolPopup";
import { useAuthorizeProdEdits } from "@common/features/data/lib/useAuthorizeProdEdits";
import { usePatchDocumentField } from "@common/features/data/components/Table/utils/usePatchDocumentField";
import {
  Table,
  TableSkeleton,
} from "@common/features/data/components/Table/Table";
import {
  DataToolbar,
  DataToolbarSkeleton,
} from "@common/features/data/components/DataToolbar/DataToolbar";
import { EmptyDataContent } from "@common/features/data/components/EmptyData";
import { useDataColumns } from "@common/features/data/components/Table/utils/useDataColumns";
import { useQueryFilteredTable } from "@common/features/data/components/Table/utils/useQueryFilteredTable";
import { useSingleTableSchemaStatus } from "@common/features/data/components/TableSchema";
import { DataFilters } from "@common/features/data/components/DataFilters/DataFilters";
import { useTableFields } from "@common/features/data/components/Table/utils/useTableFields";
import { useDefaultDocument } from "@common/features/data/lib/useDefaultDocument";
import {
  ImperativePanelHandle,
  Panel,
  PanelGroup,
} from "react-resizable-panels";
import { cn } from "@ui/cn";

import { getDefaultIndex } from "@common/features/data/components/DataFilters/IndexFilters";
import { useMount } from "react-use";
import { api } from "system-udfs/convex/_generated/api";
import { useNents } from "@common/lib/useNents";

export function DataContent({
  tableName,
  shape,
  componentId,
  activeSchema,
}: {
  tableName: string;
  componentId: string | null;
  shape: Shape | null;
  activeSchema: SchemaJson | null;
}) {
  const { filters, changeFilters, hasFilters } = useTableFilters(
    tableName,
    componentId,
  );

  const [draftFilters, setDraftFilters] = useState(filters);
  const [showFilters, setShowFilters] = useState(false);
  useEffect(() => {
    setDraftFilters(filters);
  }, [filters]);
  const router = useRouter();

  const tableSchemaStatus = useSingleTableSchemaStatus(tableName);
  const numRowsInTable = useQuery(udfs.tableSize.default, {
    tableName,
    componentId,
  });

  const hasFiltersAndAtLeastOneDocument =
    hasFilters && numRowsInTable !== undefined && numRowsInTable > 0;

  const {
    status,
    loadNextPage,
    staleAsOf,
    isLoading,
    data,
    errors,
    numRowsReadEstimate,
    isPaused,
  } = useQueryFilteredTable(tableName);

  const numRowsRead = Math.min(numRowsReadEstimate, numRowsInTable || 0);

  const { useCurrentDeployment } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();
  const isProd = deployment?.deploymentType === "prod";

  const localStorageKey =
    router.query && `${router.query.deploymentName}/${tableName}`;

  const ref = useRef<ImperativePanelHandle>(null);

  const allIds = useMemo(
    () => new Set(data.map((row) => row._id as string)),
    [data],
  );

  const selectedRows = useSelectionState(allIds, status === "Exhausted");

  const tableFields = useTableFields(tableName, shape, data);

  const columns = useDataColumns({
    tableName,
    localStorageKey,
    fields: tableFields,
    data,
    // Subtract 3 border pixels, one on each side of the parent box
    // and one more on the right side of the last column.
    width: (ref.current?.getSize() || 1000) - 3,
  });

  const listRef = useRef<FixedSizeList>(null);

  const scrollToTop = useCallback(() => listRef.current?.scrollToItem(0), []);

  const [rowsThatAreSelected, { reset: clearSelectedRows, all: allSelected }] =
    selectedRows;

  const [previousTableName, setPreviousTableName] = useState(tableName);
  if (tableName !== previousTableName) {
    setPreviousTableName(tableName);
    clearSelectedRows();
  }

  const patchDocumentField = usePatchDocumentField(tableName);

  const [areEditsAuthorized, onAuthorizeEdits] = useAuthorizeProdEdits({
    isProd,
  });

  const { addDocuments, patchFields, clearTable, deleteTable, deleteRows } =
    useDataToolbarActions({
      // Scrolling to the first item when a new document is added
      // for works now while we are guaranteed to be sorting by creation time.
      handleAddDocuments: scrollToTop,
      clearSelectedRows,
      loadMore: loadNextPage,
      tableName,
    });

  const allRowsSelected =
    allSelected === true && !hasFiltersAndAtLeastOneDocument;

  const popupState = useToolPopup({
    addDocuments: (table, docs) => addDocuments(table, docs),
    patchFields: (table, rowIds, fields) => patchFields(table, rowIds, fields),
    clearSelectedRows,
    clearTable,
    deleteRows: (rowIds) => deleteRows(rowIds),
    deleteTable,
    isProd,
    numRows: numRowsInTable,
    tableName,
    areEditsAuthorized,
    onAuthorizeEdits,
    activeSchema,
  });
  const { popupEl } = popupState;

  const selectedDocumentId = rowsThatAreSelected.values().next().value;
  const selectedDocument = data.find((row) => row._id === selectedDocumentId);
  const defaultDocument = useDefaultDocument(tableName);
  const { selectedNent } = useNents();
  const indexes =
    useQuery(api._system.frontend.indexes.default, {
      tableName,
      tableNamespace: selectedNent?.id ?? null,
    }) ?? undefined;
  const sortField =
    (
      indexes?.find((index) => index.name === filters?.index?.name)?.fields as
        | string[]
        | undefined
    )?.[0] || "_creationTime";

  return (
    <PanelGroup
      direction="horizontal"
      className={cn(
        "scrollbar flex h-full w-full min-w-[20rem] overflow-x-auto pl-6",
        popupEl ? "pr-0" : "pr-6",
      )}
      autoSaveId="data-content"
    >
      <Panel
        className={cn(
          "flex shrink flex-col gap-2 overflow-hidden py-4",
          "max-w-full",
          popupEl ? "min-w-[16rem]" : "min-w-[20rem]",
        )}
        ref={ref}
        defaultSize={80}
        minSize={10}
      >
        <DataToolbar
          popupState={popupState}
          deleteRows={deleteRows}
          selectedRowsIds={rowsThatAreSelected}
          allRowsSelected={allRowsSelected === true}
          selectedDocument={selectedDocument}
          numRows={numRowsInTable}
          tableSchemaStatus={tableSchemaStatus}
          tableName={tableName}
          isProd={isProd}
          isLoadingMore={isLoading && !isPaused}
        />

        <div className="flex h-full max-h-full flex-col overflow-y-hidden rounded-sm">
          {numRowsInTable !== undefined && numRowsInTable > 0 && (
            <DataFilters
              tableName={tableName}
              componentId={componentId}
              tableFields={tableFields}
              defaultDocument={defaultDocument}
              filters={filters}
              onChangeFilters={changeFilters}
              dataFetchErrors={errors}
              draftFilters={draftFilters}
              setDraftFilters={setDraftFilters}
              activeSchema={activeSchema}
              numRows={numRowsInTable}
              numRowsLoaded={data.length}
              hasFilters={hasFiltersAndAtLeastOneDocument}
              showFilters={showFilters}
              setShowFilters={setShowFilters}
            />
          )}

          <LoadingTransition
            loadingState={
              <div className="flex h-full flex-col items-center justify-center gap-8 rounded-sm border bg-background-secondary">
                <LoadingLogo />
              </div>
            }
            loadingProps={{ shimmer: false }}
          >
            {status !== "LoadingFirstPage" &&
              (data.length || status === "CanLoadMore" ? (
                <Sheet
                  className={classNames("w-full relative rounded-t-none")}
                  padding={false}
                >
                  {!isPaused && staleAsOf > 0 && (
                    <LoadingFilteredData
                      numRowsRead={numRowsRead}
                      numRowsInTable={numRowsInTable}
                      overlay
                    />
                  )}
                  <Table
                    activeSchema={activeSchema}
                    listRef={listRef}
                    loadMore={loadNextPage}
                    sort={{
                      order: filters?.order || "desc",
                      field: sortField,
                    }}
                    totalRowCount={
                      router.query.filters
                        ? status === "Exhausted"
                          ? data.length
                          : // If we are filtering, we need to add 1 to the total row count to
                            // allow the infinite loader to load more documents when scrolling.
                            data.length + 1
                        : numRowsInTable
                    }
                    hasFilters={hasFiltersAndAtLeastOneDocument}
                    patchDocument={patchDocumentField}
                    selectedRows={selectedRows}
                    areEditsAuthorized={areEditsAuthorized}
                    onAuthorizeEdits={onAuthorizeEdits}
                    tableName={tableName}
                    componentId={componentId}
                    isProd={isProd}
                    data={data}
                    columns={columns}
                    localStorageKey={localStorageKey}
                    hasPopup={!!popupEl}
                    setPopup={popupState.setPopup}
                    deleteRows={deleteRows}
                    defaultDocument={defaultDocument}
                    onAddDraftFilter={(filter: Filter) => {
                      setDraftFilters((prev) =>
                        prev
                          ? {
                              clauses: [...prev.clauses, filter],
                              index: prev.index ?? getDefaultIndex(),
                            }
                          : {
                              clauses: [filter],
                              index: getDefaultIndex(),
                            },
                      );
                      setShowFilters(true);
                    }}
                  />
                </Sheet>
              ) : hasFiltersAndAtLeastOneDocument ? (
                isLoading ? (
                  <Sheet
                    className="flex w-full grow animate-fadeIn items-center justify-center rounded-t-none"
                    padding={false}
                  >
                    <LoadingFilteredData
                      numRowsRead={numRowsRead}
                      numRowsInTable={numRowsInTable}
                    />
                  </Sheet>
                ) : (
                  <div className="flex h-full flex-1 flex-col items-center gap-2 rounded-t-none border bg-background-secondary pt-8">
                    <div className="text-content-secondary">
                      No documents match the selected filters.
                    </div>
                    <Button
                      onClick={() =>
                        changeFilters({
                          clauses: [],
                          index: {
                            name: filters?.index?.name || "_creationTime",
                            clauses: (filters?.index?.clauses.map((clause) => ({
                              ...clause,
                              enabled: false,
                            })) as
                              | FilterByIndex[]
                              | [...FilterByIndex[], FilterByIndexRange]) || [
                              {
                                enabled: false,
                                lowerOp: "lte",
                                lowerValue: new Date().getTime(),
                              },
                            ],
                          },
                          order: filters?.order,
                        })
                      }
                      size="xs"
                    >
                      Clear filters
                    </Button>
                  </div>
                )
              ) : isLoading ||
                (numRowsInTable !== undefined && numRowsInTable > 0) ? (
                <UnexpectedLoadingState
                  status={status}
                  numRowsInTable={numRowsInTable}
                  numRowsRead={numRowsRead}
                  isLoading={isLoading}
                />
              ) : (
                <EmptyDataContent
                  openAddDocuments={() =>
                    popupState.setPopup({ type: "addDocuments", tableName })
                  }
                />
              ))}
          </LoadingTransition>
        </div>
      </Panel>
      {popupEl}
    </PanelGroup>
  );
}

function LoadingFilteredData({
  numRowsRead,
  numRowsInTable,
  overlay = false,
}: {
  numRowsRead: number;
  numRowsInTable: any;
  overlay?: boolean;
}) {
  return (
    <div
      className={classNames(
        "flex h-full w-full items-center justify-center",
        overlay &&
          "absolute left-0 top-0 z-10 bg-white/75 dark:bg-black/75 animate-fadeIn",
      )}
    >
      <div className="flex animate-pulse flex-col items-center">
        <p>Applying filters...</p>
        <p>
          Scanned {numRowsRead.toLocaleString()} of{" "}
          {numRowsInTable?.toLocaleString()} documents
        </p>
      </div>
    </div>
  );
}

export function DataContentSkeleton() {
  return (
    <div className="flex h-full grow flex-col gap-2 p-6">
      <DataToolbarSkeleton />
      <TableSkeleton />
    </div>
  );
}

function UnexpectedLoadingState({
  status,
  numRowsInTable,
  numRowsRead,
  isLoading,
}: {
  status: string;
  numRowsInTable: number | undefined;
  numRowsRead: number;
  isLoading: boolean;
}) {
  const { captureMessage } = useContext(DeploymentInfoContext);

  useMount(() => {
    !isLoading &&
      captureMessage(
        `Encountered unexpected state in data page: status: ${status}, numRowsInTable: ${numRowsInTable}, numRowsRead: ${numRowsRead}, isLoading: ${isLoading}`,
      );
  });

  return null;
}
