import { HeaderGroup } from "react-table";
import { GenericDocument } from "convex/server";
import classNames from "classnames";
import omit from "lodash/omit";
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
  reorder,
  openContextMenu,
}: {
  reorder(item: { index: number }, newIndex: number): void;
  headerGroups: HeaderGroup<GenericDocument>[];
  isResizingColumn?: string;
  allRowsSelected: boolean | "indeterminate";
  hasFilters: boolean;
  isSelectionExhaustive: boolean;
  toggleAll: () => void;
  topBorderAnimation: boolean;
  openContextMenu: DataCellProps["onOpenContextMenu"];
}) {
  return (
    <div className="group">
      {/* Header */}
      {headerGroups.map((headerGroup) => (
        <div
          key={headerGroup.getHeaderGroupProps().key}
          {...omit(headerGroup.getHeaderGroupProps(), "key")}
          // The FixedSizeList controlling the table width somehow adds an extra pixel to the data rows,
          // so add one here too.
          className="mr-[1px] border-x border-x-transparent"
        >
          {headerGroup.headers.map((column, columnIndex) => (
            <ColumnHeader
              key={columnIndex}
              isLastColumn={columnIndex === headerGroup.headers.length - 1}
              reorder={reorder}
              isResizingColumn={isResizingColumn}
              column={column}
              columnIndex={columnIndex}
              allRowsSelected={allRowsSelected}
              hasFilters={hasFilters}
              isSelectionExhaustive={isSelectionExhaustive}
              toggleAll={toggleAll}
              openContextMenu={openContextMenu}
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
