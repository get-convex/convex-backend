import React, { useEffect, useMemo, useRef, useCallback } from "react";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { FileMetadata } from "system-udfs/convex/_system/frontend/fileStorageV2";
import {
  useReactTable,
  getCoreRowModel,
  createColumnHelper,
} from "@tanstack/react-table";
import { Loading } from "@ui/Loading";
import { EmptySection } from "@common/elements/EmptySection";
import { FileIcon } from "@radix-ui/react-icons";
import { Sheet } from "@ui/Sheet";
import { InfiniteScrollList } from "@common/elements/InfiniteScrollList";
import {
  usePaginatedFileMetadata,
  FILE_METADATA_PAGE_SIZE,
} from "../lib/usePaginatedFileMetadata";
import { FileStorageListHeader } from "./FileStorageListHeader";
import {
  FileStorageListItemInner,
  FILE_ITEM_SIZE,
} from "./FileStorageListItem";

const columnHelper = createColumnHelper<FileMetadata>();

// Custom hook to get the file columns
function useFileColumns() {
  return useMemo(
    () => [
      columnHelper.display({
        id: "select",
        size: 40,
        header: "Select",
        cell: () => (
          <span className="flex w-full justify-center pr-2">
            {/* Checkbox component rendered in the list item directly */}
          </span>
        ),
      }),
      columnHelper.accessor("_id", {
        header: "ID",
        size: 220,
        cell: () => (
          <div className="flex min-w-20 items-center gap-1">
            {/* Content rendered in the list item directly */}
          </div>
        ),
      }),
      columnHelper.accessor("size", {
        header: "Size",
        size: 90,
        cell: () => (
          <div className="min-w-20 font-mono">
            {/* Content rendered in the list item directly */}
          </div>
        ),
      }),
      columnHelper.accessor("contentType", {
        header: "Content Type",
        size: 200,
        cell: () => (
          <div className="w-full min-w-36 truncate font-mono">
            {/* Content rendered in the list item directly */}
          </div>
        ),
      }),
      columnHelper.accessor("_creationTime", {
        header: "Uploaded At",
        size: 180,
        cell: () => (
          <div className="truncate">
            {/* Content rendered in the list item directly */}
          </div>
        ),
      }),
      columnHelper.display({
        id: "actions",
        size: 90,
        header: "Actions",
        cell: () => (
          <div>
            {/* FileActions component rendered in the list item directly */}
          </div>
        ),
      }),
    ],
    [],
  );
}

export function FilesList({
  selectedFiles,
  setSelectedFiles,
  containerRef,
  totalNumFiles,
}: {
  selectedFiles: Record<Id<"_storage">, boolean>;
  setSelectedFiles: React.Dispatch<
    React.SetStateAction<Record<Id<"_storage">, boolean>>
  >;
  containerRef: React.RefObject<HTMLDivElement>;
  totalNumFiles: number | undefined;
}) {
  const {
    files: results,
    status,
    loadMore,
    isPaused,
    isLoadingPausedData,
    isRateLimited,
    togglePaused,
    reload,
  } = usePaginatedFileMetadata();

  // Remove the selection of files that no longer exist
  const prevResults = useRef<FileMetadata[]>();
  useEffect(() => {
    if (prevResults.current === results) return;
    prevResults.current = results;

    const existingFileIds = new Set(results.map((r) => r._id));

    const updatedSelectedFiles = Object.fromEntries(
      Object.entries(selectedFiles).filter(([id]) =>
        existingFileIds.has(id as Id<"_storage">),
      ),
    );

    if (
      Object.keys(updatedSelectedFiles).length !==
      Object.keys(selectedFiles).length
    ) {
      setSelectedFiles(updatedSelectedFiles);
    }
  }, [results, selectedFiles, setSelectedFiles]);

  // Calculate selection state for header
  const selectedCount = Object.values(selectedFiles).filter(Boolean).length;
  const allSelected = selectedCount === totalNumFiles && totalNumFiles! > 0;
  const someSelected = selectedCount > 0 && !allSelected;

  // Toggle all selected function
  const toggleSelectAll = useCallback(() => {
    if (allSelected || someSelected) {
      // Deselect all
      setSelectedFiles({});
    } else {
      // Select all visible files
      const newSelectedFiles = {} as Record<Id<"_storage">, boolean>;
      results.forEach((file) => {
        newSelectedFiles[file._id] = true;
      });
      setSelectedFiles(newSelectedFiles);
    }
  }, [allSelected, someSelected, results, setSelectedFiles]);

  // Setup Tanstack table
  const columns = useFileColumns();

  const tableInstance = useReactTable({
    columns,
    data: results,
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: true,
    enableMultiRowSelection: true,
    state: {
      rowSelection: selectedFiles,
    },
    getRowId: (row) => row._id,
    onRowSelectionChange: setSelectedFiles,
  });

  // Memoize item data to prevent unnecessary re-renders
  const itemData = useMemo(
    () => ({
      files: results,
      table: tableInstance,
      selectionMap: selectedFiles,
    }),
    [results, tableInstance, selectedFiles],
  );

  return (
    <div
      className="flex min-w-[37.5rem] max-w-[60rem] grow flex-col gap-4"
      ref={containerRef}
    >
      <Sheet
        padding={false}
        className="flex grow animate-fadeInFromLoading flex-col scrollbar"
      >
        <div className="flex max-h-full grow flex-col">
          <FileStorageListHeader
            isPaused={isPaused}
            isLoadingPausedData={isLoadingPausedData}
            togglePaused={togglePaused}
            isRateLimited={isRateLimited}
            reload={reload}
            allSelected={allSelected}
            someSelected={someSelected}
            toggleSelectAll={toggleSelectAll}
          />
          {status === "LoadingFirstPage" ? (
            <Loading className="max-w-[60rem]" />
          ) : results.length === 0 && status !== "CanLoadMore" ? (
            <div className="h-full max-w-[60rem]">
              <EmptySection
                sheet={false}
                Icon={FileIcon}
                color="red"
                header="No files yet."
                body="With Convex File Storage, you can store and serve files."
                learnMoreButton={{
                  href: "https://docs.convex.dev/file-storage",
                  children: "Learn more about file storage.",
                }}
              />
            </div>
          ) : (
            <div className="grow">
              <InfiniteScrollList
                className="min-w-[36.25rem] scrollbar"
                style={{
                  scrollbarGutter: "stable",
                }}
                overscanCount={25}
                outerRef={containerRef}
                items={results}
                totalNumItems={totalNumFiles}
                itemKey={(idx, data) =>
                  data.files[idx]?._id || `loading-${idx}`
                }
                itemSize={FILE_ITEM_SIZE}
                itemData={itemData}
                pageSize={FILE_METADATA_PAGE_SIZE}
                RowOrLoading={FileStorageListItemInner}
                loadMore={loadMore}
              />
            </div>
          )}
        </div>
      </Sheet>
    </div>
  );
}
