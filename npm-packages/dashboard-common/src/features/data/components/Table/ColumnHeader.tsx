import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { Tooltip } from "dashboard-common";
import { GenericDocument } from "convex/server";
import { HeaderGroup } from "react-table";
import { useDrop, useDrag } from "react-dnd";
import { useEffect, useRef } from "react";
import omit from "lodash/omit";
import { useContextMenuTrigger } from "../../lib/useContextMenuTrigger";
import { useTableDensity } from "../../lib/useTableDensity";
import { Checkbox } from "../../../../elements/Checkbox";
import { identifierNeedsEscape } from "../../lib/helpers";
import { emptyColumnName } from "./utils/useDataColumns";
import { DataCellProps } from "./DataCell/DataCell";
import { columnWidthToString } from "./DataRow";

type ColumnHeaderProps = {
  column: HeaderGroup<GenericDocument>;
  columnIndex: number;
  allRowsSelected: boolean | "indeterminate";
  hasFilters: boolean;
  isSelectionExhaustive: boolean;
  toggleAll: () => void;
  reorder(item: { index: number }, newIndex: number): void;
  isResizingColumn?: string;
  isLastColumn: boolean;
  openContextMenu: DataCellProps["onOpenContextMenu"];
};

export function ColumnHeader({
  reorder,
  column,
  columnIndex,
  allRowsSelected = false,
  hasFilters,
  isSelectionExhaustive,
  toggleAll,
  isResizingColumn,
  isLastColumn,
  openContextMenu,
}: ColumnHeaderProps) {
  const canDragOrDrop = columnIndex !== 0 && !isResizingColumn;

  const { ref, isDragging, isHovering, direction } = useColumnDragAndDrop(
    column,
    columnIndex,
    reorder,
    canDragOrDrop,
  );
  const columnName = column.Header as string;
  useContextMenuTrigger(
    ref,
    (pos) =>
      openContextMenu(pos, null, {
        column: columnName,
        value: undefined,
      }),
    () => {},
  );

  const { densityValues } = useTableDensity();
  const width = columnWidthToString(column.getHeaderProps().style?.width);
  return (
    <div
      key={column.getHeaderProps().key}
      {...omit(column.getHeaderProps({ style: { width } }), "key")}
      className={classNames(
        canDragOrDrop && "cursor-grab hover:bg-background-primary",
        isDragging && "bg-background-tertiary cursor-grabbing",
        "font-semibold group/headerCell text-left text-xs bg-background-secondary text-content-secondary tracking-wider",
        "select-none duration-300 transition-colors",
        "border-r",
        isResizingColumn === columnName && "border-r-util-accent",
      )}
    >
      {/* Show a border on the side the column will be dropped */}
      {!isDragging && isHovering && direction && (
        <div
          className={classNames(
            "absolute top-[1px] h-full w-[2px] bg-border-selected",
            direction === "left" ? "left-0" : "right-0",
          )}
        />
      )}
      <div
        className="flex items-center space-x-2"
        ref={ref}
        style={{
          padding: `${densityValues.paddingY}px ${columnIndex === 0 ? "12" : densityValues.paddingX}px`,
          width,
        }}
      >
        {columnIndex === 0 ? (
          // Disable the "Select all" checkbox when filtering
          allRowsSelected === false &&
          hasFilters &&
          !isSelectionExhaustive ? null : (
            <Checkbox checked={allRowsSelected} onChange={toggleAll} />
          )
        ) : column.Header === emptyColumnName ? (
          <i>empty</i>
        ) : typeof column.Header === "string" &&
          identifierNeedsEscape(column.Header) ? (
          <span
            className={`before:text-content-primary before:content-['"'] after:text-content-primary after:content-['"']`}
          >
            {column.render("Header")}
          </span>
        ) : (
          <div>{column.render("Header")}</div>
        )}
        {!column.disableResizing && (
          <div
            {...column.getResizerProps()}
            className="absolute top-0 z-20 inline-block h-full"
            style={{
              // @ts-expect-error bad typing in react-table
              ...column.getResizerProps().style,
              width: densityValues.paddingX * (isLastColumn ? 1 : 2),
              right: isLastColumn ? 0 : -densityValues.paddingX,
            }}
          />
        )}
        {column.Header !== "_creationTime" &&
          (column as unknown as { isDate: boolean }).isDate && (
            <Tooltip
              tip="Displaying numbers as dates. Hover or edit the cell by double-clicking see the unformatted value."
              side="top"
              align="start"
              className="flex items-center"
            >
              <QuestionMarkCircledIcon />
            </Tooltip>
          )}
      </div>
    </div>
  );
}

export function useColumnDragAndDrop(
  column: HeaderGroup<GenericDocument>,
  columnIndex: number,
  reorder: (item: { index: number }, newIndex: number) => void,
  canDragOrDrop: boolean,
) {
  const ref = useRef<HTMLDivElement>(null);
  const { id } = column;
  const [{ isHovering, offset }, drop] = useDrop({
    accept: "column",
    canDrop: () => canDragOrDrop,
    drop: (item: { index: number }) => {
      reorder(item, columnIndex);
    },
    collect: (monitor) => ({
      isHovering: canDragOrDrop && monitor.isOver({ shallow: true }),
      offset: monitor.getDifferenceFromInitialOffset(),
    }),
  });

  const direction = offset?.x ? (offset.x > 0 ? "right" : "left") : undefined;

  const [{ isDragging }, drag, preview] = useDrag({
    type: "column",
    canDrag: canDragOrDrop,
    item: () => ({
      id,
      index: columnIndex,
    }),
    collect: (monitor) => ({
      isDragging: monitor.isDragging(),
    }),
  });

  useEffect(() => {
    preview(ref);
  }, [preview]);

  drag(drop(ref));

  return { ref, direction, isDragging, isHovering };
}
