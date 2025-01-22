import { Value } from "convex/values";
import { GenericDocument } from "convex/server";
import React, {
  CSSProperties,
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Row } from "react-table";
import classNames from "classnames";
import { useFirstMountState, usePrevious } from "react-use";
import { areEqual } from "react-window";
import { Loading, SchemaJson } from "dashboard-common";
import { useContextMenuTrigger } from "elements/useContextMenuTrigger";
import { Target } from "elements/ContextMenu";
import { useTableDensity } from "hooks/useTableDensity";
import { cn } from "lib/cn";
import omit from "lodash/omit";
import { TableCheckbox } from "./TableCheckbox";
import { DataCell, DataCellProps } from "./DataCell/DataCell";
import { usePatchDocumentField } from "./utils/usePatchDocumentField";
import { arrowKeyHandler } from "./utils/arrowKeyHandler";
import { toggleAdjacent } from "./utils/toggleAdjacent";

type DataRowProps = {
  data: {
    areEditsAuthorized: boolean;
    isRowSelected(row: string): boolean;
    isSelectionAllNonExhaustive: boolean;
    resizingColumn: string | undefined;
    onAuthorizeEdits?(): void;
    patchDocument: ReturnType<typeof usePatchDocumentField>;
    prepareRow: (row: Row) => void;
    rows: Row[];
    tableName: string;
    toggleIsRowSelected(key: string): void;
    onOpenContextMenu: DataCellProps["onOpenContextMenu"];
    onCloseContextMenu: () => void;
    contextMenuRow: string | null;
    contextMenuColumn: string | null;
    canManageTable: boolean;
    activeSchema: SchemaJson | null;
    onEditDocument: (document: GenericDocument) => void;
  };
  index: number;
  style: CSSProperties;
};

export const DataRow = memo(DataRowImpl, areEqual);

function DataRowImpl(props: DataRowProps) {
  const { data, index, style } = props;

  const firstRow = data.rows.length ? data.rows[0] : undefined;
  firstRow && data.prepareRow(firstRow);
  const { densityValues } = useTableDensity();
  return index >= data.rows.length ? (
    <div
      className="ml-[1px] border-b"
      style={{ ...style, height: densityValues.height }}
    >
      <Loading>
        {firstRow ? (
          firstRow.cells.map((cell) => (
            <div {...cell.getCellProps()} className="h-full border-r" />
          ))
        ) : (
          <div className="ml-4 mt-4 h-4 w-full rounded bg-neutral-8/20 dark:bg-neutral-3/20" />
        )}
      </Loading>
    </div>
  ) : (
    <DataRowLoaded {...props} />
  );
}

export type EditingColumn =
  | {
      document: GenericDocument;
      column: string;
      editedValue: Value;
    }
  | undefined;

function DataRowLoaded({ index, style, data }: DataRowProps) {
  const {
    areEditsAuthorized,
    isRowSelected,
    isSelectionAllNonExhaustive,
    onAuthorizeEdits,
    patchDocument,
    prepareRow,
    rows,
    tableName,
    toggleIsRowSelected,
    onOpenContextMenu,
    onCloseContextMenu,
    canManageTable,
    activeSchema,
    resizingColumn,
    onEditDocument,
    contextMenuColumn,
    contextMenuRow,
  } = data;

  const row: Row = rows[index];
  const previousRow = usePrevious(row);
  const previousRows = usePrevious(rows);

  const didNumberOfRowsChange = previousRows?.length !== rows.length;

  const { _id } = row.values;
  const previousRowId = previousRow?.values._id;

  const [didJustCreate, setDidJustCreate] = useState(false);
  useEffect(() => {
    // The entire row should be highlighted if the row was recently created and
    // not already rendered.
    if (!previousRowId && Date.now() - row.values._creationTime < 1000) {
      setDidJustCreate(true);
      // To reset the animatation, reset the state after one second.
      setTimeout(() => setDidJustCreate(false), 1000);
    }
  }, [row, previousRow, previousRowId, _id]);

  const mounting = useFirstMountState();
  const checked = isRowSelected(_id);
  prepareRow(row);

  // Context menu trigger for the checkbox cell
  const checkboxRef = useRef<HTMLLabelElement | null>(null);
  const contextMenuCallback = useCallback(
    (position: Target) => onOpenContextMenu(position, _id, null),
    [onOpenContextMenu, _id],
  );
  useContextMenuTrigger(checkboxRef, contextMenuCallback, onCloseContextMenu);
  const document = useMemo(() => omit(row.values, "*select"), [row.values]);

  const editDocument = useCallback(() => {
    canManageTable && onEditDocument(document);
  }, [canManageTable, onEditDocument, document]);

  return (
    <div
      className={classNames(
        // Make sure the focus ring is visible on first and last cell
        "focus:ring-none focus:border",
        didJustCreate && "animate-highlight",
        "DataRow",
        checked && "bg-background-tertiary",
      )}
      {...row.getRowProps({
        style,
      })}
      key={row.getRowProps().key}
    >
      {row.cells.map((cell, columnIndex) => {
        const width = columnWidthToString(cell.getCellProps().style?.width);
        return (
          <div
            {...cell.getCellProps({ style: { width } })}
            key={cell.getCellProps().key}
            className={cn(
              "border-r transition-colors duration-300",
              resizingColumn === (cell.column.Header as string) &&
                "border-r-util-accent",
            )}
          >
            {columnIndex === 0 ? (
              <TableCheckbox
                width={width}
                ref={checkboxRef}
                onKeyDown={arrowKeyHandler(checkboxRef)}
                isSelectionAllNonExhaustive={isSelectionAllNonExhaustive}
                onToggle={() => toggleIsRowSelected(_id)}
                onToggleAdjacent={() =>
                  toggleAdjacent(
                    rows.map((r) => r.values._id),
                    index,
                    isRowSelected,
                    toggleIsRowSelected,
                  )
                }
                checked={checked}
              />
            ) : (
              <DataCell
                activeSchema={activeSchema}
                rowId={_id}
                document={document}
                didRowChange={
                  // The row changed if it's already been mounted,
                  // the previous row is not the same as the current row,
                  // and the number of rows has not changed.
                  !mounting && previousRowId !== _id && !didNumberOfRowsChange
                }
                areEditsAuthorized={areEditsAuthorized}
                onAuthorizeEdits={onAuthorizeEdits}
                editDocument={editDocument}
                value={cell.value}
                column={cell.column}
                width={width}
                inferIsDate={
                  (cell.column as unknown as { isDate: boolean }).isDate
                }
                patchDocument={patchDocument}
                tableName={tableName}
                onOpenContextMenu={onOpenContextMenu}
                onCloseContextMenu={onCloseContextMenu}
                isContextMenuOpen={
                  contextMenuColumn === (cell.column.Header as string) &&
                  contextMenuRow === _id
                }
                canManageTable={canManageTable}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

// The goal here is to floor the width of the column to the nearest pixel to avoid
// sub-pixel rendering issues in the browser.
export const columnWidthToString = (width?: string | number) =>
  width
    ? `${Math.floor(
        typeof width === "string" ? Number(width.replace("px", "")) : width,
      ).toString()}px`
    : `0px`;
