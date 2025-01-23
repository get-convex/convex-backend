import { GenericDocument } from "convex/server";
import classNames from "classnames";
import {
  MutableRefObject,
  useCallback,
  useContext,
  useReducer,
  useRef,
  useState,
} from "react";
import {
  Column,
  useBlockLayout,
  useColumnOrder,
  useResizeColumns,
  useTable,
} from "react-table";
import { FixedSizeList } from "react-window";
import {
  Filter,
  SchemaJson,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { DndProvider } from "react-dnd";
import { HTML5Backend } from "react-dnd-html5-backend";
import withScrolling from "react-dnd-scrolling";
import { DeploymentInfoContext, LoadingLogo, useNents } from "dashboard-common";
import {
  ImperativePanelHandle,
  Panel,
  PanelGroup,
} from "react-resizable-panels";
import { cn } from "lib/cn";
import { ResizeHandle } from "../../../../layouts/SidebarDetailLayout";
import { useTableDensity } from "../../lib/useTableDensity";
import { InfiniteScrollList } from "../../../../elements/InfiniteScrollList";
import { SelectionState } from "../../lib/useSelectionState";
import { usePatchDocumentField } from "./utils/usePatchDocumentField";
import { DataRow } from "./DataRow";
import { TableScrollbar } from "./TableScrollbar";
import { useTrackColumnWidths } from "./utils/useTrackColumnWidths";
import type { PopupState } from "../../lib/useToolPopup";
import { useMaintainScrollPositionOnChange } from "./utils/useMaintainScrollPositionOnChange";
import { TableContextMenu, useTableContextMenuState } from "./TableContextMenu";
import { TableHeader } from "./TableHeader";
import { useStoredColumnOrder } from "./utils/useDataColumns";
import { ViewDocument } from "./ViewDocument";
import { pageSize } from "./utils/useQueryFilteredTable";

const ScrollingComponent = withScrolling("div");

const getRowId = (d: GenericDocument) => d._id as string;

const pageThreshold = 25;

export function Table({
  activeSchema,
  columns = [],
  data = [],
  localStorageKey = "_disabled_",
  areEditsAuthorized,
  onAuthorizeEdits,
  hasFilters,
  selectedRows,
  loadMore,
  totalRowCount,
  listRef,
  patchDocument,
  tableName,
  componentId,
  isProd,
  setPopup,
  deleteRows,
  onAddDraftFilter,
  defaultDocument,
}: {
  activeSchema: SchemaJson | null;
  areEditsAuthorized: boolean;
  onAuthorizeEdits?(): void;
  columns: Column<GenericDocument>[];
  data: GenericDocument[]; // array of row data so far
  localStorageKey?: string;
  tableName: string;
  componentId: string | null;
  isProd: boolean;
  selectedRows: SelectionState;
  totalRowCount?: number;
  hasFilters: boolean;
  loadMore: () => void;
  listRef: MutableRefObject<FixedSizeList | null>;
  patchDocument: ReturnType<typeof usePatchDocumentField>;
  setPopup: PopupState["setPopup"];
  deleteRows: (rowIds: Set<string>) => Promise<void>;
  onAddDraftFilter: (newFilter: Filter) => void;
  defaultDocument: GenericDocument;
}) {
  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );

  const { selectedNent } = useNents();

  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );
  const canManageTable =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;

  const [storedColumnOrder, setStoredColumnOrder] =
    useStoredColumnOrder(localStorageKey);

  const dataColumnNames = columns.map((c) => c.Header as string);

  const {
    state,
    getTableProps,
    getTableBodyProps,
    headerGroups,
    rows,
    prepareRow,
    setColumnOrder,
  } = useTable(
    {
      columns,
      data,
      getRowId,
      autoResetSortBy: false,
      initialState: {
        columnOrder: storedColumnOrder || dataColumnNames,
      },
      stateReducer: (newState, action) => {
        if (action.type === "setColumnOrder") {
          setStoredColumnOrder(newState.columnOrder);
        }

        return newState;
      },
    },
    useBlockLayout,
    useColumnOrder,
    useResizeColumns,
  );

  trackDataColumnChanges(dataColumnNames, storedColumnOrder, setColumnOrder);

  const reorderColumns = (item: { index: number }, newIndex: number) => {
    const { index: currentIndex } = item;

    const currentItem = state.columnOrder[currentIndex];

    const newColumnOrder = [...state.columnOrder];
    newColumnOrder.splice(currentIndex, 1);
    newColumnOrder.splice(newIndex, 0, currentItem);
    setColumnOrder(newColumnOrder);
  };

  const resetColumnWidths = useTrackColumnWidths(state, localStorageKey);

  const { isResizingColumn } = state.columnResizing;

  const [
    ,
    {
      has: isRowSelected,
      toggle: toggleIsRowSelected,
      toggleAll,
      all: allRowsSelected,
      isExhaustive: isSelectionExhaustive,
    },
  ] = selectedRows;

  const outerRef = useRef<HTMLElement>(null);

  const [, forceRerender] = useReducer((x) => x + 1, 0);

  const [topBorderAnimation, setTopBorderAnimation] = useState(false);
  const animateTopBorder = useCallback(() => {
    setTopBorderAnimation(true);
    setTimeout(() => setTopBorderAnimation(false), 1000);
  }, []);

  const { densityValues } = useTableDensity();

  useMaintainScrollPositionOnChange(
    data,
    outerRef,
    getRowId,
    densityValues.height,
    animateTopBorder,
  );

  const { contextMenuState, openContextMenu, closeContextMenu } =
    useTableContextMenuState();
  const [collapsed, setCollapsed] = useState(false);

  const panelRef = useRef<ImperativePanelHandle>(null);

  const onEditDocument = useCallback(
    (document: GenericDocument) => {
      setPopup({
        type: "editDocument",
        document,
      });
    },
    [setPopup],
  );

  return (
    <PanelGroup
      direction="horizontal"
      className="w-full"
      autoSaveId="documentViewer"
    >
      <Panel defaultSize={100} className="relative w-full">
        <DndProvider backend={HTML5Backend}>
          <ScrollingComponent
            {...getTableProps()}
            className={classNames(
              "flex rounded w-full h-full overflow-y-hidden",
              "scrollbar",
            )}
          >
            <div className="flex flex-auto flex-col">
              <TableHeader
                key={state.columnOrder.join(",")}
                reorder={reorderColumns}
                headerGroups={headerGroups}
                isResizingColumn={isResizingColumn}
                allRowsSelected={allRowsSelected}
                hasFilters={hasFilters}
                isSelectionExhaustive={isSelectionExhaustive}
                toggleAll={toggleAll}
                topBorderAnimation={topBorderAnimation}
                openContextMenu={openContextMenu}
              />
              {/* Body */}
              <div
                {...getTableBodyProps()}
                className="mt-[-1px] w-full flex-auto"
                id="dataTable"
              >
                <InfiniteScrollList
                  className="scrollbar-none"
                  items={rows}
                  totalNumItems={totalRowCount}
                  pageSize={pageSize}
                  loadMoreThreshold={pageThreshold}
                  itemSize={densityValues.height}
                  itemData={{
                    areEditsAuthorized,
                    isRowSelected,
                    isSelectionAllNonExhaustive:
                      !isSelectionExhaustive && allRowsSelected === true,
                    onAuthorizeEdits,
                    patchDocument,
                    prepareRow,
                    rows,
                    tableName,
                    toggleIsRowSelected,
                    onOpenContextMenu: openContextMenu,
                    onCloseContextMenu: closeContextMenu,
                    contextMenuRow:
                      contextMenuState?.selectedCell?.rowId ?? null,
                    contextMenuColumn:
                      contextMenuState?.selectedCell?.column ?? null,
                    canManageTable: canManageTable && !isInUnmountedComponent,
                    activeSchema,
                    resizingColumn: isResizingColumn,
                    onEditDocument,
                  }}
                  RowOrLoading={DataRow}
                  loadMore={loadMore}
                  listRef={listRef}
                  outerRef={outerRef}
                  onScroll={() => {
                    // Force a re-render so the TableScrollbar gets updated.
                    forceRerender();
                  }}
                  itemKey={(idx) => (data[idx] as any)?._id || idx}
                />
              </div>
              <TableScrollbar
                totalRowCount={totalRowCount}
                outerRef={outerRef}
                listRef={listRef}
              />
            </div>

            <TableContextMenu
              data={data}
              state={contextMenuState}
              close={closeContextMenu}
              isProd={isProd}
              setPopup={setPopup}
              deleteRows={deleteRows}
              onAddDraftFilter={onAddDraftFilter}
              defaultDocument={defaultDocument}
              canManageTable={canManageTable}
              resetColumns={() => {
                setColumnOrder(dataColumnNames);
                setStoredColumnOrder(dataColumnNames);
                resetColumnWidths();
              }}
            />
          </ScrollingComponent>
        </DndProvider>
      </Panel>
      {selectedRows[0].size > 0 && (
        <>
          <ResizeHandle
            collapsed={collapsed}
            direction="left"
            panelRef={panelRef}
          />
          <Panel
            defaultSize={30}
            minSize={10}
            maxSize={80}
            collapsible
            collapsedSize={0}
            onCollapse={() => setCollapsed(true)}
            onExpand={() => setCollapsed(false)}
            ref={panelRef}
            className={cn(
              "relative overflow-x-auto",
              !collapsed && "min-w-[14rem]",
            )}
          >
            <ViewDocument
              rows={data.filter((d) =>
                Array.from(selectedRows[0]).includes(d._id as string),
              )}
              columns={dataColumnNames}
              tableName={tableName}
              componentId={componentId}
              canManageTable={canManageTable}
              areEditsAuthorized={areEditsAuthorized}
              onAuthorizeEdits={onAuthorizeEdits}
              activeSchema={activeSchema}
            />
          </Panel>
        </>
      )}
    </PanelGroup>
  );
}

export function TableSkeleton() {
  return (
    <div className="flex h-full items-center justify-center rounded bg-background-secondary">
      <LoadingLogo />
    </div>
  );
}

// Checks if there are any new or removed columns in the data set
// and updates the column order accordingly.
function trackDataColumnChanges(
  columns: string[],
  storedColumnOrder: string[] | undefined,
  updateColumnOrder: (newOrder: string[]) => void,
) {
  if (!storedColumnOrder) {
    return;
  }

  // Find columns that are not in the stored order and add them to the end
  const newColumns = columns
    .filter((c) => !storedColumnOrder.includes(c))
    // New columns should be sorted alphabetically after the existing sort order.
    .sort((a, b) => a.localeCompare(b));

  const existingColumns = storedColumnOrder.filter((c) => columns.includes(c));

  // There are no new columns and no columns have been removed, so we don't need to make an update.
  if (
    newColumns.length === 0 &&
    existingColumns.length === storedColumnOrder.length
  ) {
    return;
  }

  const lastColumn = existingColumns[existingColumns.length - 1];

  // If _creationTime is the last column, we should respect that and
  //  insert the new columns before it.
  const newOrder =
    lastColumn === "_creationTime"
      ? [
          ...existingColumns.filter((c) => c !== "_creationTime"),
          ...newColumns,
          "_creationTime",
        ]
      : [...existingColumns, ...newColumns];

  updateColumnOrder(newOrder);
}
