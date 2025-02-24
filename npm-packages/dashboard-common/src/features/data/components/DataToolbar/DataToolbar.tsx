import {
  Pencil1Icon,
  PlusIcon,
  QuestionMarkCircledIcon,
  TrashIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { useContext } from "react";
import { Button } from "@common/elements/Button";
import { LoadingTransition } from "@common/elements/Loading";
import { Tooltip } from "@common/elements/Tooltip";
import { Spinner } from "@common/elements/Spinner";
import { useShowGlobalRunner } from "@common/features/functionRunner/lib/functionRunner";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useNents } from "@common/lib/useNents";
import { PopupState } from "@common/features/data/lib/useToolPopup";
import { useEnabledDebounced } from "@common/features/data/lib/useEnabledDebounced";
import { FilterButton } from "@common/features/data/components/DataFilters/FilterButton";
import { DataOverflowMenu } from "@common/features/data/components/DataOverflowMenu/DataOverflowMenu";
import {
  isTableMissingFromSchema,
  useActiveSchema,
} from "@common/features/data/lib/helpers";
import { TableSchemaStatus } from "@common/features/data/components/TableSchema";

export type DataToolbarProps = {
  popupState: PopupState;
  allRowsSelected: boolean;
  deleteRows: (rowIds: Set<string>) => Promise<void>;
  filters: FilterExpression | undefined;
  hasFilters: boolean;
  isLoadingMore: boolean;
  isProd: boolean;
  tableSchemaStatus: TableSchemaStatus | undefined;
  numRows?: number;
  numRowsLoaded: number;
  selectedRowsIds: Set<string>;
  selectedDocument: Record<string, any> | undefined;
  tableName: string;
  showFilters: boolean;
  setShowFilters: (show: boolean) => void;
};

export function DataToolbar({
  popupState: { popup: popupState, setPopup },
  allRowsSelected,
  filters,
  deleteRows,
  hasFilters,
  isLoadingMore,
  isProd,
  tableSchemaStatus,
  numRows,
  numRowsLoaded,
  selectedRowsIds,
  selectedDocument,
  tableName,
  showFilters,
  setShowFilters,
}: DataToolbarProps) {
  const popup = popupState?.type;

  const showSpinner = useEnabledDebounced(isLoadingMore);

  const schema = useActiveSchema();
  const isMissingFromSchema = isTableMissingFromSchema(tableName, schema);

  const { selectedNent } = useNents();

  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );

  const numRowsSelected = selectedRowsIds.size;
  const selectionToolsEnabled = numRowsSelected > 0 || allRowsSelected;

  const showGlobalRunner = useShowGlobalRunner();

  const isEditingAllAndMoreThanOne = allRowsSelected && numRowsSelected !== 1;
  const isEditingMoreThanOne =
    isEditingAllAndMoreThanOne || numRowsSelected > 1;

  const numRowsWeKnowOf = hasFilters ? numRowsLoaded : numRows;

  const {
    useLogDeploymentEvent,
    useCurrentDeployment,
    useHasProjectAdminPermissions,
  } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();

  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canManageTable =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;

  return (
    <div className="flex flex-col">
      <div className="flex flex-wrap items-end justify-between gap-4">
        {/* Left side of the toolbar. */}
        <div className="flex max-w-full items-center gap-4">
          <div className="flex max-w-full flex-col gap-1">
            <Tooltip
              side="right"
              tip={
                isMissingFromSchema
                  ? "This table is not defined in your schema."
                  : undefined
              }
            >
              <h3 className="flex items-start gap-0.5 font-mono">
                {tableName}{" "}
                {isMissingFromSchema && (
                  <span className="font-sans text-base">*</span>
                )}
              </h3>
            </Tooltip>
            <LoadingTransition
              loadingProps={{
                fullHeight: false,
                className: "h-4",
              }}
            >
              {numRowsWeKnowOf !== undefined && (
                <div
                  className={classNames(
                    "flex items-center gap-1",
                    "text-xs whitespace-nowrap",
                    "text-content-secondary",
                  )}
                >
                  <span className="font-semibold">
                    {numRowsWeKnowOf.toLocaleString()}{" "}
                  </span>
                  {numRowsWeKnowOf === 1 ? "document" : "documents"}{" "}
                  {hasFilters && (
                    <>
                      {numRowsWeKnowOf !== numRows && `loaded`}
                      <Tooltip
                        tip="Filtered results are paginated and more documents will be loaded as you scroll."
                        side="right"
                      >
                        <QuestionMarkCircledIcon />
                      </Tooltip>
                    </>
                  )}
                </div>
              )}
            </LoadingTransition>
          </div>
          <div className="w-4">
            <div
              hidden={!showSpinner}
              aria-hidden={!showSpinner}
              aria-label="Loading more documents..."
            >
              <Spinner />
            </div>
          </div>
        </div>
        {/* Right side of the toolbar. */}
        <div className="flex flex-wrap items-center gap-2">
          {(!selectionToolsEnabled || popup === "addDocuments") && (
            <AddDocumentButton
              popup={popup}
              popupState={popupState}
              setPopup={setPopup}
              tableName={tableName}
              canManageTable={canManageTable}
              isInUnmountedComponent={isInUnmountedComponent}
              log={log}
            />
          )}
          {selectionToolsEnabled ||
          ((popup === "editDocument" || popup === "bulkEdit") &&
            popupState?.tableName === tableName) ? (
            <EditDocumentButton
              popupState={popupState}
              setPopup={setPopup}
              tableName={tableName}
              canManageTable={canManageTable}
              isInUnmountedComponent={isInUnmountedComponent}
              log={log}
              isEditingMoreThanOne={isEditingMoreThanOne}
              allRowsSelected={allRowsSelected}
              numRowsSelected={numRowsSelected}
              selectedRowsIds={selectedRowsIds}
              selectedDocument={selectedDocument}
            />
          ) : null}
          {selectionToolsEnabled && (
            <DeleteDocumentButton
              popup={popup}
              setPopup={setPopup}
              tableName={tableName}
              canManageTable={canManageTable}
              isInUnmountedComponent={isInUnmountedComponent}
              log={log}
              isEditingAllAndMoreThanOne={isEditingAllAndMoreThanOne}
              allRowsSelected={allRowsSelected}
              numRowsSelected={numRowsSelected}
              selectedRowsIds={selectedRowsIds}
              isProd={isProd}
              deleteRows={deleteRows}
            />
          )}

          <FilterButton
            filters={filters}
            open={showFilters}
            onClick={() => {
              log(
                showFilters ? "collapse data filters" : "expand data filters",
                {
                  how: "toolbar",
                },
              );
              setShowFilters(!showFilters);
            }}
          />
          {tableSchemaStatus && (
            <DataOverflowMenu
              tableSchemaStatus={tableSchemaStatus}
              numRows={numRows ?? 0}
              onClickCustomQuery={() =>
                showGlobalRunner(
                  { type: "customQuery", table: tableName },
                  "click",
                )
              }
              onClickClearTable={() => {
                log("open delete document panel", {
                  how: "toolbar",
                  count: "all",
                });
                setPopup({ type: "clearTable", tableName });
              }}
              onClickSchemaIndexes={() => {
                log("view table schema", {
                  how: "toolbar",
                });
                setPopup({ type: "viewSchema", tableName });
              }}
              onClickMetrics={() => {
                log("view table metrics", {
                  how: "toolbar",
                });
                setPopup({ type: "metrics", tableName });
              }}
              onClickDeleteTable={() => {
                log("open delete table panel", {
                  how: "toolbar",
                });
                setPopup({ type: "deleteTable", tableName });
              }}
            />
          )}
        </div>
      </div>
    </div>
  );
}

export function documentsLabel(numDocuments: number, allRowsSelected: boolean) {
  if (!allRowsSelected && numDocuments === 0) {
    return "";
  }
  return allRowsSelected && numDocuments !== 1
    ? "All"
    : numDocuments === 1
      ? ""
      : numDocuments;
}

export function DataToolbarSkeleton() {
  return (
    <div className="flex justify-between">
      <div className="flex h-12 flex-col gap-2">
        <div className="h-6 w-36 rounded bg-neutral-8/20 dark:bg-neutral-3/10" />
        <div className="h-4 w-16 rounded bg-neutral-8/20 dark:bg-neutral-3/10" />
      </div>
      <div className="flex h-[2.375rem] gap-2">
        <div className="w-36 rounded bg-neutral-8/20 dark:bg-neutral-3/10" />
        <div className="w-28 rounded bg-neutral-8/20 dark:bg-neutral-3/10" />
        <div className="w-[2.375rem] rounded bg-neutral-8/20 dark:bg-neutral-3/10" />
      </div>
    </div>
  );
}

type AddDocumentButtonProps = {
  popup: string | undefined;
  popupState: PopupState["popup"];
  setPopup: PopupState["setPopup"];
  tableName: string;
  canManageTable: boolean;
  isInUnmountedComponent: boolean;
  log: (event: string, data: Record<string, any>) => void;
};

function AddDocumentButton({
  popup,
  popupState,
  setPopup,
  tableName,
  canManageTable,
  isInUnmountedComponent,
  log,
}: AddDocumentButtonProps) {
  const isAddingInCurrentlyViewedTable =
    popupState?.type === "addDocuments" && popupState.tableName === tableName;

  return (
    <Button
      onClick={() => {
        if (
          (popup === "addDocuments" || popup === "editDocument") &&
          isAddingInCurrentlyViewedTable
        ) {
          setPopup(undefined);
          return;
        }
        log("open add documents panel", { how: "toolbar" });
        setPopup({ type: "addDocuments", tableName });
      }}
      size="sm"
      variant="neutral"
      focused={popup === "addDocuments" && isAddingInCurrentlyViewedTable}
      icon={<PlusIcon aria-hidden="true" />}
      disabled={!canManageTable || isInUnmountedComponent}
      tip={
        isInUnmountedComponent
          ? "Cannot add documents in an unmounted component."
          : !canManageTable &&
            "You do not have permission to add documents in production."
      }
    >
      Add
    </Button>
  );
}

type EditDocumentButtonProps = {
  popupState: PopupState["popup"];
  setPopup: PopupState["setPopup"];
  tableName: string;
  canManageTable: boolean;
  isInUnmountedComponent: boolean;
  log: (event: string, data: Record<string, any>) => void;
  isEditingMoreThanOne: boolean;
  allRowsSelected: boolean;
  numRowsSelected: number;
  selectedRowsIds: Set<string>;
  selectedDocument: Record<string, any> | undefined;
};

function EditDocumentButton({
  popupState,
  setPopup,
  tableName,
  canManageTable,
  isInUnmountedComponent,
  log,
  isEditingMoreThanOne,
  allRowsSelected,
  numRowsSelected,
  selectedRowsIds,
  selectedDocument,
}: EditDocumentButtonProps) {
  const isPopupFocused = (() => {
    if (!popupState) return false;

    if (popupState.type !== "bulkEdit" && popupState.type !== "editDocument") {
      return false;
    }

    if (popupState.tableName !== tableName) {
      return false;
    }

    if (popupState.type === "editDocument") {
      return (
        selectedRowsIds.size === 1 &&
        popupState.document._id === selectedDocument?._id
      );
    }

    if (popupState.type === "bulkEdit") {
      return popupState.rowIds === "all"
        ? allRowsSelected
        : popupState.rowIds === selectedRowsIds;
    }

    return false;
  })();

  return (
    <Button
      disabled={!canManageTable || isInUnmountedComponent}
      tip={
        isInUnmountedComponent
          ? "Cannot edit documents in an unmounted component."
          : !canManageTable &&
            "You do not have permission to edit documents in production."
      }
      size="sm"
      variant="neutral"
      onClick={() => {
        if (isPopupFocused) {
          setPopup(undefined);
          return;
        }
        log("open document editor", {
          how: "toolbar",
          count: allRowsSelected ? "all" : numRowsSelected,
        });

        if (isEditingMoreThanOne) {
          setPopup({
            type: "bulkEdit",
            rowIds: allRowsSelected ? "all" : selectedRowsIds,
            tableName,
          });
        } else {
          setPopup({
            type: "editDocument",
            document: selectedDocument!,
            tableName,
          });
        }
      }}
      focused={isPopupFocused}
      icon={<Pencil1Icon aria-hidden="true" />}
    >
      Edit {documentsLabel(numRowsSelected, allRowsSelected)}
    </Button>
  );
}

type DeleteDocumentButtonProps = {
  popup: string | undefined;
  setPopup: PopupState["setPopup"];
  tableName: string;
  canManageTable: boolean;
  isInUnmountedComponent: boolean;
  log: (event: string, data: Record<string, any>) => void;
  isEditingAllAndMoreThanOne: boolean;
  allRowsSelected: boolean;
  numRowsSelected: number;
  selectedRowsIds: Set<string>;
  isProd: boolean;
  deleteRows: (rowIds: Set<string>) => Promise<void>;
};

function DeleteDocumentButton({
  popup,
  setPopup,
  tableName,
  canManageTable,
  isInUnmountedComponent,
  log,
  isEditingAllAndMoreThanOne,
  allRowsSelected,
  numRowsSelected,
  selectedRowsIds,
  isProd,
  deleteRows,
}: DeleteDocumentButtonProps) {
  return (
    <Button
      disabled={!canManageTable || isInUnmountedComponent}
      tip={
        isInUnmountedComponent
          ? "Cannot delete documents in an unmounted component."
          : !canManageTable &&
            "You do not have permission to delete documents in production."
      }
      onClick={async () => {
        log("open delete document panel", {
          how: "toolbar",
          count: allRowsSelected ? "all" : numRowsSelected,
        });

        if (isEditingAllAndMoreThanOne) {
          setPopup({ type: "clearTable", tableName });
        } else if (isProd) {
          setPopup({
            type: "deleteRows",
            rowIds: selectedRowsIds,
          });
        } else {
          await deleteRows(selectedRowsIds);
        }
      }}
      size="sm"
      variant="danger"
      focused={popup === "deleteRows"}
      icon={<TrashIcon aria-hidden="true" />}
    >
      Delete {documentsLabel(numRowsSelected, allRowsSelected)}
    </Button>
  );
}
