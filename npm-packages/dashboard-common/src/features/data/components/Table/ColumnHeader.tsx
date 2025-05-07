import { CaretUpIcon, QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { GenericDocument } from "convex/server";
import { HeaderGroup } from "react-table";
import { useDrop, useDrag } from "react-dnd";
import { useContext, useEffect, useRef } from "react";
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

  return (
    <div
      key={column.getHeaderProps().key}
      {...omit(column.getHeaderProps({ style: { width } }), "key")}
      className={classNames(
        canDragOrDrop && "cursor-grab hover:bg-background-primary",
        isDragging && "bg-background-tertiary cursor-grabbing",
        "font-semibold group/headerCell text-left text-xs bg-background-secondary text-content-secondary tracking-wider",
        "select-none duration-300 transition-colors",
        !isLastColumn && "border-r",
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
      <ValidatorTooltip
        fieldSchema={fieldSchema}
        columnName={columnName}
        disableTooltip={!!isResizingColumn}
      >
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
      </ValidatorTooltip>
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
