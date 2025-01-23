import {
  CaretDownIcon,
  CopyIcon,
  DownloadIcon,
  FileIcon,
  TrashIcon,
  UploadIcon,
} from "@radix-ui/react-icons";
import * as Sentry from "@sentry/nextjs";
import {
  Button,
  buttonClasses,
  Loading,
  Tooltip,
  Spinner,
  NentSwitcher,
  useNents,
  toast,
  formatBytes,
  useCopy,
  Sheet,
  ConfirmationDialog,
  EmptySection,
  Checkbox,
} from "dashboard-common";
import { useMutation, usePaginatedQuery, useQuery } from "convex/react";
import Link from "next/link";
import React, { useEffect, useMemo, useRef, useState } from "react";
import udfs from "udfs";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { FileMetadata } from "system-udfs/convex/_system/frontend/fileStorageV2";
import Image from "next/image";
import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import {
  useReactTable,
  getCoreRowModel,
  flexRender,
  createColumnHelper,
} from "@tanstack/react-table";

const columnHelper = createColumnHelper<FileMetadata>();

export function FileStorageContent() {
  const [selectedFiles, setSelectedFiles] = useState<
    Record<Id<"_storage">, boolean>
  >({});
  const selectedFilesArr = Object.keys(selectedFiles).filter(
    (key) => selectedFiles[key as Id<"_storage">],
  ) as Id<"_storage">[];

  const [isDraggingFile, setIsDraggingFile] = useState(false);
  const useUploadFilesResult = useUploadFiles();

  return (
    <div
      className="relative flex h-full flex-col gap-4 overflow-hidden p-6 py-4"
      onDragOver={(e) => {
        e.preventDefault();
        if (e.dataTransfer.types.includes("Files")) {
          setIsDraggingFile(true);
        }
      }}
      onDragLeave={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDraggingFile(false);
      }}
      onDrop={(e) => {
        e.preventDefault();
        setIsDraggingFile(false);

        const { handleUpload, isUploading, cantUploadFilesReason } =
          useUploadFilesResult;

        if (isUploading) {
          toast(
            "error",
            "Cannot upload files while another upload is in progress.",
          );
          return;
        }

        if (cantUploadFilesReason) {
          toast("error", cantUploadFilesReason);
          return;
        }

        void handleUpload(e.dataTransfer.files);
      }}
    >
      <div className="flex flex-col">
        <div className="w-fit min-w-60">
          <NentSwitcher />
        </div>
        <FileStorageHeader
          selectedFiles={selectedFilesArr}
          useUploadFilesResult={useUploadFilesResult}
        />
      </div>
      <Files
        selectedFiles={selectedFiles}
        setSelectedFiles={setSelectedFiles}
      />
      {isDraggingFile && (
        // eslint-disable-next-line no-restricted-syntax
        <div className="pointer-events-none absolute inset-0 z-50 mx-6 my-4 flex max-w-[46rem] animate-fadeInFromLoading items-center justify-center rounded-lg border-2 border-dashed bg-background-secondary/70 text-center text-lg tracking-tight text-content-tertiary backdrop-blur-sm">
          <UploadIcon className="mr-2 size-6" />
          Drop files to upload
        </div>
      )}
    </div>
  );
}

function FileStorageHeader({
  selectedFiles,
  useUploadFilesResult,
}: {
  selectedFiles: Id<"_storage">[];
  useUploadFilesResult: ReturnType<typeof useUploadFiles>;
}) {
  const numFiles = useQuery(udfs.fileStorageV2.numFiles, {
    componentId: useNents().selectedNent?.id ?? null,
  });
  return (
    <div className="flex w-full min-w-fit max-w-[46rem] flex-wrap items-center justify-between gap-2">
      <div className="flex items-center gap-4">
        <div className="flex flex-1 flex-col gap-1">
          <h3>File Storage</h3>
          <div
            className="flex items-center gap-1 text-xs text-content-secondary"
            data-testid="fileCount"
          >
            <span className="text-xs">Total Files</span>
            {numFiles !== undefined && (
              <span className="font-semibold">{numFiles.toLocaleString()}</span>
            )}
          </div>
        </div>
      </div>

      <div className="flex items-start gap-2">
        <DeleteFilesButton selectedFiles={selectedFiles} />
        <Uploader useUploadFilesResult={useUploadFilesResult} />
      </div>
    </div>
  );
}

export function DeleteFilesButton({
  selectedFiles,
}: {
  selectedFiles: Id<"_storage">[];
}) {
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );

  const { selectedNent } = useNents();
  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );
  const canDeleteFiles =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;

  const { length } = selectedFiles;
  if (length === 0) return null;

  return (
    <>
      <Button
        onClick={() => setShowDeleteModal(true)}
        variant="danger"
        icon={<TrashIcon aria-hidden="true" />}
        disabled={!canDeleteFiles || isInUnmountedComponent}
        tip={
          isInUnmountedComponent
            ? "Cannot delete files in an unmounted component."
            : !canDeleteFiles &&
              "You do not have permission to delete files in production"
        }
      >
        Delete {`${length} file${length > 1 ? "s" : ""}`}
      </Button>

      {showDeleteModal && (
        <DeleteFileModal
          storageIds={selectedFiles}
          onClose={() => setShowDeleteModal(false)}
        />
      )}
    </>
  );
}

export function useUploadFiles() {
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );

  const canUploadFiles =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;
  const generateUploadUrl = useMutation(udfs.fileStorageV2.generateUploadUrl);

  const [isUploading, setIsUploading] = useState(false);

  const { selectedNent } = useNents();
  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );

  async function handleSingleUpload(file: File): Promise<{
    status: "success" | "failure";
    name: string;
  }> {
    try {
      const postUrl = await generateUploadUrl({
        componentId: selectedNent?.id ?? null,
      });
      const response = await fetch(postUrl, {
        method: "POST",
        headers: file!.type ? { "Content-Type": file!.type } : undefined,
        body: file,
      });
      if (!response.ok) {
        throw new Error(
          `Failed to upload ${response.status} ${response.statusText}`,
        );
      }
      return { status: "success", name: file.name };
    } catch (err) {
      Sentry.captureException(err);
      return { status: "failure", name: file.name };
    }
  }

  async function handleUpload(files: FileList) {
    const beforeUnload = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      // eslint-disable-next-line no-param-reassign
      event.returnValue = "File upload is in progress";
    };

    window.addEventListener("beforeunload", beforeUnload);

    setIsUploading(true);

    const results = await Promise.all(
      Array.from(files).map((file) => handleSingleUpload(file)),
    );

    const successes = results.filter((x) => x.status === "success");
    if (successes.length > 0) {
      toast(
        "success",
        `${
          successes.length === 1
            ? `File “${successes[0].name}”`
            : `${successes.length} files`
        } uploaded.`,
      );
    }

    const failures = results.filter((x) => x.status === "failure");
    if (failures.length > 0) {
      toast(
        "error",
        `Failed to upload ${
          failures.length === 1
            ? `file “${failures[0].name}”`
            : `${failures.length} files`
        }, please try again.`,
      );
    }

    setIsUploading(false);
    window.removeEventListener("beforeunload", beforeUnload);
  }

  return {
    handleUpload,
    isUploading,
    cantUploadFilesReason: isInUnmountedComponent
      ? "Cannot upload files in an unmounted component."
      : !canUploadFiles
        ? "You do not have permission to upload files in production."
        : null,
  };
}

export function Uploader({
  useUploadFilesResult,
}: {
  useUploadFilesResult: ReturnType<typeof useUploadFiles>;
}) {
  const { handleUpload, isUploading, cantUploadFilesReason } =
    useUploadFilesResult;

  const fileInput = useRef<HTMLInputElement>(null);

  return (
    <div className="flex flex-col items-center justify-center gap-2">
      <Tooltip wrapsButton tip={cantUploadFilesReason} side="left">
        <label
          htmlFor="uploader"
          aria-disabled={isUploading || cantUploadFilesReason !== null}
          className={buttonClasses({
            className: "ml-auto",
            size: "sm",
            variant: "primary",
            disabled: isUploading || cantUploadFilesReason !== null,
          })}
        >
          {/* This needs to be wrapped in a dom element to 
            fix an issue with the google translate extension
            throwing errors when the icon switches between the loading and upload icon
            https://github.com/facebook/react/issues/11538#issuecomment-390386520
         */}
          <div>{isUploading ? <Spinner /> : <UploadIcon />}</div>
          Upload Files
          <input
            disabled={isUploading || cantUploadFilesReason !== null}
            id="uploader"
            data-testid="uploader"
            type="file"
            onChange={async (event) => {
              const { files } = event.target;

              if (files !== null) {
                await handleUpload(files);
              }

              if (fileInput.current) {
                fileInput.current.value = "";
              }
            }}
            ref={fileInput}
            className="hidden"
            multiple
          />
        </label>
      </Tooltip>
      <p className="text-xs text-content-tertiary">or drag files here</p>
    </div>
  );
}

function Files({
  selectedFiles,
  setSelectedFiles,
}: {
  selectedFiles: Record<Id<"_storage">, boolean>;
  setSelectedFiles: React.Dispatch<
    React.SetStateAction<Record<Id<"_storage">, boolean>>
  >;
}) {
  const { results, status, loadMore } = usePaginatedQuery(
    udfs.fileStorageV2.fileMetadata,
    { componentId: useNents().selectedNent?.id ?? null },
    {
      initialNumItems: 20,
    },
  );

  const data = useMemo(() => results, [results]);

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

  const copyId = useCopy("Storage ID");

  const columns = useMemo(
    () => [
      columnHelper.display({
        id: "select",
        size: 36,
        header: ({ table }) => (
          <span className="flex w-full justify-center">
            <Checkbox
              {...{
                checked: table.getIsSomeRowsSelected()
                  ? "indeterminate"
                  : table.getIsAllRowsSelected(),
                onChange: table.getToggleAllRowsSelectedHandler(),
              }}
            />
          </span>
        ),
        cell: ({ row }) => (
          <span className="flex w-full justify-center">
            <Checkbox
              {...{
                checked: row.getIsSelected(),
                onChange: row.getToggleSelectedHandler(),
              }}
            />
          </span>
        ),
      }),
      columnHelper.accessor("_id", {
        header: "Storage ID",
        minSize: 56,
        maxSize: 300,
        cell: ({ getValue }) => (
          <div className="flex items-center">
            <div className="truncate font-mono text-content-primary">
              {getValue()}
            </div>
            <Button
              tip="Copy Storage ID to clipboard"
              tipSide="bottom"
              aria-label="Copy Storage ID"
              onClick={() => copyId(getValue())}
              className="float-right text-content-secondary"
              size="xs"
              variant="neutral"
              inline
              icon={<CopyIcon />}
            />
          </div>
        ),
      }),
      columnHelper.accessor("size", {
        header: () => <div className="w-full pr-3 text-right">Size</div>,
        size: 100,
        cell: ({ getValue }) => (
          <div className="pr-3 text-right font-mono">
            {formatBytes(Number(getValue()))}
          </div>
        ),
      }),
      columnHelper.accessor("contentType", {
        header: "Content Type",
        minSize: 100,
        maxSize: 200,
        cell: ({ getValue }) => (
          <div className="w-full truncate font-mono">
            {getValue() || "Unknown"}
          </div>
        ),
      }),
      columnHelper.accessor("_creationTime", {
        header: () => (
          <div className="flex items-center gap-1">
            Uploaded At <CaretDownIcon />
          </div>
        ),
        size: 160,
        cell: ({ getValue }) => (
          <div className="truncate text-left">
            {new Date(getValue()).toLocaleString()}
          </div>
        ),
      }),
      columnHelper.display({
        id: "actions",
        size: 80,
        header: () => <div className="sr-only w-full">Actions</div>,
        cell: ({ row }) => <FileActions file={row.original} />,
      }),
    ],
    [copyId],
  );

  const tableInstance = useReactTable({
    columns,
    data,
    getCoreRowModel: getCoreRowModel(),
    columnResizeMode: "onEnd",
    enableColumnResizing: true,
    enableRowSelection: true,
    enableMultiRowSelection: true,
    state: {
      rowSelection: selectedFiles,
    },
    getRowId: (row) => row._id,
    onRowSelectionChange: setSelectedFiles,
  });

  if (status === "LoadingFirstPage") {
    return <Loading />;
  }

  if (results.length === 0 && status !== "CanLoadMore") {
    return (
      <div className="h-full max-w-[46rem]">
        <EmptySection
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
    );
  }

  const headerClass =
    "text-left text-xs text-content-secondary font-normal py-2";

  return (
    <div className="flex min-w-[24rem] max-w-[46rem] flex-col gap-4 overflow-hidden">
      <Sheet
        padding={false}
        className="flex grow animate-fadeInFromLoading flex-col overflow-auto scrollbar"
      >
        <table className="text-xs">
          <thead className="sticky top-0 flex w-full min-w-fit items-center overflow-hidden border-b bg-background-secondary">
            {tableInstance.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <th
                    colSpan={header.colSpan}
                    key={header.column.id}
                    style={{ width: `${header.getSize()}px` }}
                    className={headerClass}
                  >
                    {flexRender(
                      header.column.columnDef.header,
                      header.getContext(),
                    )}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody className="space-y-2">
            {tableInstance.getRowModel().rows?.map((row) => (
              <tr
                key={row.id}
                className="group flex items-center"
                data-testid="filerow"
              >
                {row.getAllCells().map((cell) => (
                  <td
                    key={cell.id}
                    style={{ width: `${cell.column.getSize()}px` }}
                  >
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </Sheet>
      <div className="ml-auto w-fit">
        {status === "CanLoadMore" && (
          <Button
            onClick={() => {
              loadMore(20);
            }}
            size="sm"
            variant="neutral"
          >
            Load More
          </Button>
        )}
      </div>
    </div>
  );
}

function FileActions({ file }: { file: FileMetadata }) {
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canDeleteFiles =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;
  const { selectedNent } = useNents();
  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );

  const [showDeleteModal, setShowDeleteModal] = useState(false);

  return (
    <div className="flex h-full gap-1">
      <Tooltip
        side="right"
        tip={
          file.contentType?.startsWith("image") &&
          // Only render image previews for files under ~10mb
          file.size < 10_000_000 && <PreviewImage url={file.url} />
        }
      >
        <Link
          href={file.url}
          passHref
          className={buttonClasses({
            variant: "primary",
            size: "sm",
            inline: true,
          })}
          aria-label="Download File"
          download
          target="_blank"
        >
          <DownloadIcon aria-label="Download" />
        </Link>
      </Tooltip>
      <Button
        aria-label="Delete File"
        variant="danger"
        inline
        size="sm"
        disabled={!canDeleteFiles || isInUnmountedComponent}
        tip={
          isInUnmountedComponent
            ? "Cannot delete files in an unmounted component."
            : !canDeleteFiles &&
              "You do not have permission to delete files in production."
        }
        onClick={() => setShowDeleteModal(true)}
        icon={<TrashIcon />}
      />
      {showDeleteModal && (
        <DeleteFileModal
          storageIds={[file._id]}
          onClose={() => setShowDeleteModal(false)}
        />
      )}
    </div>
  );
}

function DeleteFileModal({
  storageIds,
  onClose,
}: {
  storageIds: Id<"_storage">[];
  onClose: () => void;
}) {
  const deleteFiles = useMutation(udfs.fileStorageV2.deleteFiles);
  const { selectedNent } = useNents();
  const handleDelete = async () => {
    await deleteFiles({ storageIds, componentId: selectedNent?.id ?? null });
  };

  return (
    <ConfirmationDialog
      onClose={onClose}
      onConfirm={handleDelete}
      confirmText="Delete"
      dialogTitle="Delete File"
      dialogBody={
        storageIds.length === 1 ? (
          <>
            Are you sure you want delete file{" "}
            <code className="rounded bg-background-tertiary p-1 text-sm text-content-secondary">
              {storageIds[0]}
            </code>
            ? Deleted files cannot be recovered.
          </>
        ) : (
          <>
            Are you sure you want delete {storageIds.length} files? Deleted
            files cannot be recovered.
          </>
        )
      }
    />
  );
}

function PreviewImage({ url }: { url: string }) {
  const [[width, height], setSize] = useState<
    [number | undefined, number | undefined]
  >([undefined, undefined]);
  return (
    <div className="relative">
      {(!width || !height) && <Spinner className="animate-fadeInFromLoading" />}
      <Image
        src={url}
        alt="image preview"
        // Hack to correctly size the preview image to the appropriate dimensions:
        // Start with fill set to true, then set the width and height to the natural width and height of the image.
        fill={!width || !height}
        objectFit="contain"
        width={width}
        height={height}
        onLoadingComplete={({ naturalWidth, naturalHeight }) => {
          setSize([naturalWidth, naturalHeight]);
        }}
      />
    </div>
  );
}
