import {
  CaretUpIcon,
  QuestionMarkCircledIcon,
  DragHandleDots2Icon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { GenericDocument } from "convex/server";
import { HeaderGroup } from "react-table";
import { useDrop, useDrag } from "react-dnd";
import { useRef, useContext, useState } from "react";
import omit from "lodash/omit";
import { useContextMenuTrigger } from "@common/features/data/lib/useContextMenuTrigger";
import { useTableDensity } from "@common/features/data/lib/useTableDensity";
import { Checkbox } from "@ui/Checkbox";
import { identifierNeedsEscape } from "@common/features/data/lib/helpers";
import { emptyColumnName } from "@common/features/data/components/Table/utils/useDataColumns";
import { DataCellProps } from "@common/features/data/components/Table/DataCell/DataCell";
import { columnWidthToString } from "@common/features/data/components/Table/DataRow";
import { Tooltip } from "@ui/Tooltip";
import { cn } from "@ui/cn";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { documentValidatorForTable } from "@common/features/data/components/Table/utils/validators";
import { Button } from "@ui/Button";
import { ValidatorTooltip } from "./ValidatorTooltip";

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
  sort?: "asc" | "desc";
  activeSchema: any | null;
  tableName: string;
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
  sort,
  activeSchema,
  tableName,
}: ColumnHeaderProps) {
  const canDragOrDrop = columnIndex !== 0 && !isResizingColumn;

  const headerNode = useRef<HTMLDivElement | null>(null);

  const { isDragging, isHovering, direction, drop, drag, dragPreview } =
    useColumnDragAndDrop(column, columnIndex, reorder, canDragOrDrop);
  const columnName = column.Header as string;
  useContextMenuTrigger(
    headerNode,
    (pos) =>
      openContextMenu(pos, null, {
        column: columnName,
        value: undefined,
      }),
    () => {},
  );

  const { enableIndexFilters } = useContext(DeploymentInfoContext);

  const { densityValues } = useTableDensity();
  const width = columnWidthToString(column.getHeaderProps().style?.width);

  // Get the validator information for the tooltip
  const documentValidator =
    activeSchema && documentValidatorForTable(activeSchema, tableName);
  const fieldSchema =
    documentValidator?.type === "object"
      ? documentValidator.value[columnName]
      : undefined;

  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      key={column.getHeaderProps().key}
      {...omit(column.getHeaderProps({ style: { width } }), "key")}
      className={classNames(
        isDragging && "cursor-grabbing",
        "font-semibold text-left text-xs bg-background-secondary text-content-secondary tracking-wider",
        "select-none duration-300 transition-colors",
        !isLastColumn && "border-r",
        isResizingColumn === columnName && "border-r-util-accent",
        "relative",
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Show a border on the side the column will be dropped */}
      {!isDragging && isHovering && direction && (
        <div
          className={classNames(
            "absolute top-px h-full w-px bg-util-accent",
            direction === "left" ? "left-0" : "right-0",
          )}
        />
      )}
      <ValidatorTooltip
        fieldSchema={fieldSchema}
        columnName={columnName}
        disableTooltip={!!isResizingColumn}
      >
        <div
          ref={(node) => {
            headerNode.current = node;
            if (node) {
              drop(node);
              dragPreview(node);
            }
          }}
          className={cn(
            "flex w-full items-center justify-between space-x-2",
            isDragging && "cursor-grabbing",
          )}
          style={{
            padding: `${densityValues.paddingY}px ${columnIndex === 0 ? "12" : densityValues.paddingX}px`,
            width,
          }}
        >
          <div className="flex items-center space-x-2">
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
            {sort && enableIndexFilters && (
              <Tooltip tip="You may change the sort order in the Filter & Sort menu.">
                <CaretUpIcon
                  className={cn(
                    "transition-all",
                    sort === "asc" ? "" : "rotate-180",
                  )}
                />
              </Tooltip>
            )}
          </div>
          {canDragOrDrop && isHovered && (
            <Button
              ref={(node) => {
                node && drag(node);
              }}
              className={cn(
                "absolute right-1.5 animate-fadeInFromLoading cursor-grab items-center bg-background-secondary/50 text-content-secondary backdrop-blur-[2px]",
                isDragging && "cursor-grabbing",
              )}
              aria-label="Drag column"
              variant="neutral"
              inline
              size="xs"
              onKeyDown={(e) => {
                if (e.key === " " || e.key === "Enter") {
                  e.preventDefault();
                  // Optionally, trigger drag start here if needed for keyboard users
                }
              }}
              icon={<DragHandleDots2Icon />}
            />
          )}
        </div>
      </ValidatorTooltip>
      {!isHovering && !column.disableResizing && (
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
    </div>
  );
}

export function useColumnDragAndDrop(
  column: HeaderGroup<GenericDocument>,
  columnIndex: number,
  reorder: (item: { index: number }, newIndex: number) => void,
  canDragOrDrop: boolean,
) {
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

  const [{ isDragging }, drag, dragPreview] = useDrag({
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

  return { isDragging, isHovering, direction, drop, drag, dragPreview };
}
