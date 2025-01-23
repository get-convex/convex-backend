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

import {
  LoadingLogo,
  LoadingTransition,
  Button,
  Sheet,
  DeploymentInfoContext,
} from "dashboard-common";
import { useQuery } from "convex/react";
import udfs from "udfs";
import classNames from "classnames";
import {
  Filter,
  FilterExpression,
  SchemaJson,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Shape } from "shapes";
import { useSelectionState } from "../lib/useSelectionState";
import { useDataToolbarActions } from "../lib/useDataToolbarActions";
import { useTableFilters } from "../lib/useTableFilters";
import { useToolPopup } from "../lib/useToolPopup";
import { useAuthorizeProdEdits } from "../lib/useAuthorizeProdEdits";
import { usePatchDocumentField } from "./Table/utils/usePatchDocumentField";
import { Table, TableSkeleton } from "./Table/Table";
import { DataToolbar, DataToolbarSkeleton } from "./DataToolbar/DataToolbar";
import { EmptyDataContent } from "./EmptyData";
import { useDataColumns } from "./Table/utils/useDataColumns";
import { useQueryFilteredTable } from "./Table/utils/useQueryFilteredTable";
import { useSingleTableSchemaStatus } from "./TableSchema";
import { DataFilters } from "./DataFilters/DataFilters";
import { useTableFields } from "./Table/utils/useTableFields";
import { useDefaultDocument } from "../lib/useDefaultDocument";

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
  const [showFilters, setShowFilters] = useState(false);
  const [draftFilters, setDraftFilters] = useState(filters);
  useEffect(() => {
    setDraftFilters(filters);
  }, [filters]);
  const router = useRouter();

  const tableSchemaStatus = useSingleTableSchemaStatus(tableName);
  const numRowsInTable = useQuery(udfs.tableSize.default, {
    tableName,
    componentId,
  });
  const {
    status,
    loadNextPage,
    staleAsOf,
    isUsingFilters,
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

  const ref = useRef<HTMLDivElement>(null);

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
    width: (ref.current?.clientWidth || 1000) - 3,
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
      tableName,
      // Scrolling to the first item when a new document is added
      // for works now while we are guaranteed to be sorting by creation time.
      handleAddDocuments: scrollToTop,
      clearSelectedRows,
      loadMore: loadNextPage,
    });

  const allRowsSelected = allSelected === true && !hasFilters;

  const popupState = useToolPopup({
    addDocuments,
    allRowsSelected,
    patchFields,
    clearSelectedRows,
    clearTable,
    deleteRows,
    deleteTable,
    isProd,
    numRows: numRowsInTable,
    numRowsSelected: rowsThatAreSelected.size,
    tableName,
    areEditsAuthorized,
    onAuthorizeEdits,
    activeSchema,
  });
  const { popupEl } = popupState;

  const selectedDocumentId = rowsThatAreSelected.values().next().value;
  const selectedDocument = data.find((row) => row._id === selectedDocumentId);
  const defaultDocument = useDefaultDocument(tableName);
  const changeFilterAndMaybeCloseThem = (newFilters: FilterExpression) => {
    void changeFilters(newFilters);
    if (newFilters.clauses.length === 0) {
      setShowFilters(false);
    }
  };

  return (
    <div
      className="flex h-full min-w-[30rem] flex-1 flex-col gap-6 overflow-hidden p-6 py-4 transition-all"
      ref={ref}
    >
      {popupEl}
      <div className="flex flex-col gap-2">
        <DataToolbar
          popupState={popupState}
          deleteRows={deleteRows}
          selectedRowsIds={rowsThatAreSelected}
          allRowsSelected={allRowsSelected === true}
          selectedDocument={selectedDocument}
          numRows={numRowsInTable}
          numRowsLoaded={data.length || 0}
          tableSchemaStatus={tableSchemaStatus}
          tableName={tableName}
          filters={filters}
          hasFilters={hasFilters}
          isProd={isProd}
          isLoadingMore={isLoading && !isPaused}
          showFilters={showFilters}
          setShowFilters={setShowFilters}
        />
        {showFilters && (
          <DataFilters
            tableName={tableName}
            componentId={componentId}
            tableFields={tableFields}
            defaultDocument={defaultDocument}
            filters={filters}
            onChangeFilters={changeFilterAndMaybeCloseThem}
            dataFetchErrors={errors}
            draftFilters={draftFilters}
            setDraftFilters={setDraftFilters}
            activeSchema={activeSchema}
          />
        )}
      </div>

      <LoadingTransition
        loadingState={
          <div className="flex h-full flex-col items-center justify-center gap-8 rounded border bg-background-secondary">
            <LoadingLogo />
          </div>
        }
        loadingProps={{ shimmer: false }}
      >
        {status !== "LoadingFirstPage" &&
          (data.length || status === "CanLoadMore" ? (
            <Sheet className={classNames("w-full relative")} padding={false}>
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
                totalRowCount={
                  router.query.filters
                    ? status === "Exhausted"
                      ? data.length
                      : // If we are filtering, we need to add 1 to the total row count to
                        // allow the infinite loader to load more documents when scrolling.
                        data.length + 1
                    : numRowsInTable
                }
                hasFilters={hasFilters}
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
                setPopup={popupState.setPopup}
                deleteRows={deleteRows}
                defaultDocument={defaultDocument}
                onAddDraftFilter={(filter: Filter) => {
                  setShowFilters(true);
                  setDraftFilters((prev) =>
                    prev
                      ? { clauses: [...prev.clauses, filter] }
                      : { clauses: [filter] },
                  );
                }}
              />
            </Sheet>
          ) : isUsingFilters ? (
            isLoading ? (
              <Sheet
                className="flex w-full grow animate-fadeIn items-center justify-center"
                padding={false}
              >
                <LoadingFilteredData
                  numRowsRead={numRowsRead}
                  numRowsInTable={numRowsInTable}
                />
              </Sheet>
            ) : (
              <div className="flex h-full flex-1 flex-col items-center gap-2 pt-8">
                <div className="text-content-secondary">
                  No documents match the selected filters.
                </div>
                <Button
                  onClick={() => changeFilters({ clauses: [] })}
                  size="sm"
                >
                  Clear filters
                </Button>
              </div>
            )
          ) : (
            <EmptyDataContent
              openAddDocuments={() =>
                popupState.setPopup({ type: "addDocuments" })
              }
            />
          ))}
      </LoadingTransition>
    </div>
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
    <div className="flex h-full grow flex-col gap-6 p-6">
      <DataToolbarSkeleton />
      <TableSkeleton />
    </div>
  );
}
