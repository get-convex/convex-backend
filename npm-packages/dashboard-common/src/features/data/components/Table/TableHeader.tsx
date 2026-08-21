import { HeaderGroup } from "@tanstack/react-table";
import { GenericDocument } from "convex/server";
import classNames from "classnames";
import { RefObject } from "react";
import { ColumnHeader } from "@common/features/data/components/Table/ColumnHeader";
import { DataCellProps } from "@common/features/data/components/Table/DataCell/DataCell";

export function TableHeader({
  headerGroups,
  isResizingColumn,
  allRowsSelected,
  hasFilters,
  isSelectionExhaustive,
  toggleAll,
  topBorderAnimation,
  openContextMenu,
  sort,
  localStorageKey,
  tableContainerRef,
}: {
  headerGroups: HeaderGroup<GenericDocument>[];
  isResizingColumn?: string;
  allRowsSelected: boolean | "indeterminate";
  hasFilters: boolean;
  isSelectionExhaustive: boolean;
  toggleAll: () => void;
  topBorderAnimation: boolean;
  openContextMenu: DataCellProps["onOpenContextMenu"];
  sort: {
    order: "asc" | "desc";
    field: string;
  };
  localStorageKey: string;
  tableContainerRef: RefObject<HTMLDivElement>;
}) {
  return (
    <div className="group">
      {/* Header */}
      {headerGroups.map((headerGroup) => (
        <div
          key={headerGroup.id}
          role="row"
          // The FixedSizeList controlling the table width somehow adds an extra pixel to the data rows,
          // so add one here too.
          className="mr-px flex border-x border-x-transparent"
        >
          {headerGroup.headers.map((header, columnIndex) => (
            <ColumnHeader
              key={columnIndex}
              isLastColumn={columnIndex === headerGroup.headers.length - 1}
              isResizingColumn={isResizingColumn}
              header={header}
              columnIndex={columnIndex}
              allRowsSelected={allRowsSelected}
              hasFilters={hasFilters}
              isSelectionExhaustive={isSelectionExhaustive}
              toggleAll={toggleAll}
              openContextMenu={openContextMenu}
              sort={sort.field === header.column.id ? sort.order : undefined}
              localStorageKey={localStorageKey}
              tableContainerRef={tableContainerRef}
            />
          ))}
        </div>
      ))}
      <div
        className={classNames(
          "h-[1px] bg-border-transparent",
          topBorderAnimation && "animate-highlightBorder",
        )}
      />
    </div>
  );
}
