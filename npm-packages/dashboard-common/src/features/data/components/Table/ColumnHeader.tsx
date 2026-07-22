import {
  CaretUpIcon,
  CalendarIcon,
  DragHandleDots2Icon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { GenericDocument } from "convex/server";
import { HeaderGroup } from "react-table";
import { useSortable } from "@dnd-kit/sortable";
import { useRef, useState, RefObject } from "react";
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
import { Button } from "@ui/Button";
import { useStoredShowFieldsAsDates } from "@common/features/data/components/Table/utils/useDataColumns";

type ColumnHeaderProps = {
  column: HeaderGroup<GenericDocument>;
  columnIndex: number;
  allRowsSelected: boolean | "indeterminate";
  hasFilters: boolean;
  isSelectionExhaustive: boolean;
  toggleAll: () => void;
  isResizingColumn?: string;
  isLastColumn: boolean;
  openContextMenu: DataCellProps["onOpenContextMenu"];
  sort?: "asc" | "desc";
  localStorageKey: string;
  tableContainerRef: RefObject<HTMLDivElement>;
};

export function ColumnHeader({
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
  localStorageKey,
  tableContainerRef,
}: ColumnHeaderProps) {
  const canDragOrDrop = columnIndex !== 0 && !isResizingColumn;

  const headerNode = useRef<HTMLDivElement | null>(null);

  const columnName = column.Header as string;
  const columnId = column.id;

  const { attributes, listeners, setNodeRef, isDragging, isOver, active } =
    useSortable({
      id: columnId,
      disabled: !canDragOrDrop,
    });

  // Always drop to the right of the hovered column
  const direction =
    isOver && !isDragging && active && active.id !== columnId
      ? "right"
      : undefined;
  const isHovering = isOver && !isDragging && active?.id !== columnId;
  useContextMenuTrigger(
    headerNode,
    (pos) =>
      openContextMenu(pos, null, {
        column: columnName,
        value: undefined,
      }),
    () => {},
  );

  const { densityValues } = useTableDensity();
  const width = columnWidthToString(column.getHeaderProps().style?.width);

  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      key={column.getHeaderProps().key}
      {...omit(
        column.getHeaderProps({
          style: { width, height: densityValues.height },
        }),
        "key",
      )}
      ref={setNodeRef}
      className={classNames(
        isDragging && "opacity-50",
        "font-semibold text-left text-xs bg-background-secondary text-content-secondary tracking-wider",
        "select-none duration-300 transition-colors",
        "border-r",
        "relative",
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Show a vertical line on the right side where the column will be dropped */}
      {isHovering && direction && (
        <div
          className="absolute top-0 right-0 z-10 w-0.5 bg-util-accent"
          style={{
            height: tableContainerRef.current?.offsetHeight || "100%",
          }}
        />
      )}
      {/* Show a vertical line when resizing this column */}
      {isResizingColumn === columnName && (
        <div
          className="absolute top-0 right-0 z-10 w-0.5 bg-util-accent"
          style={{
            height: tableContainerRef.current?.offsetHeight || "100%",
          }}
        />
      )}
      <div
        ref={headerNode}
        className="flex w-full items-center space-x-2"
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
            (column as unknown as { isDateLike?: boolean }).isDateLike && (
              <DateDisplayToggle
                columnName={columnName}
                isDate={(column as unknown as { isDate: boolean }).isDate}
                localStorageKey={localStorageKey}
              />
            )}
          {sort && (
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
            {...attributes}
            {...listeners}
            className={cn(
              "absolute right-1.5 animate-fadeInFromLoading cursor-grab items-center bg-background-secondary/50 text-content-secondary backdrop-blur-[2px]",
              isDragging && "cursor-grabbing",
            )}
            aria-label="Drag column"
            variant="neutral"
            inline
            size="xs"
            icon={<DragHandleDots2Icon />}
          />
        )}
      </div>
      {!isHovering && !column.disableResizing && columnName !== "*select" && (
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

// Lets the user override, per deployment → table → column, whether
// timestamp-like numbers in this column are displayed as dates. Clicking the
// icon toggles the choice, which is persisted in local storage (see
// `useStoredShowFieldsAsDates`).
function DateDisplayToggle({
  columnName,
  isDate,
  localStorageKey,
}: {
  columnName: string;
  isDate: boolean;
  localStorageKey: string;
}) {
  const [showFieldsAsDates, setShowFieldsAsDates] =
    useStoredShowFieldsAsDates(localStorageKey);

  return (
    <Button
      variant="unstyled"
      className="flex items-center text-content-secondary hover:text-content-primary"
      aria-label={
        isDate ? "Show this field as a number" : "Show this field as a date"
      }
      tip="Switch between rendering this field as a number or date."
      onClick={() =>
        setShowFieldsAsDates({ ...showFieldsAsDates, [columnName]: !isDate })
      }
      icon={
        isDate ? (
          <CalendarIcon />
        ) : (
          <span
            aria-hidden
            className="flex size-[15px] items-center justify-center font-mono text-sm leading-none font-semibold"
          >
            #
          </span>
        )
      }
    />
  );
}
