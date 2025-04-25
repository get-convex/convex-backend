import { UploadIcon } from "@radix-ui/react-icons";
import { useQuery } from "convex/react";
import React, { useRef, useState } from "react";
import udfs from "@common/udfs";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { toast } from "@common/lib/utils";
import { NentSwitcher } from "@common/elements/NentSwitcher";
import { useNents } from "@common/lib/useNents";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { PageContent } from "@common/elements/PageContent";
import { useUploadFiles } from "./Uploader";
import { FileStorageHeader } from "./FileStorageHeader";
import { FilesList } from "./FilesList";

export function FileStorageView() {
  const [selectedFiles, setSelectedFiles] = useState<
    Record<Id<"_storage">, boolean>
  >({});
  const selectedFilesArr = Object.keys(selectedFiles).filter(
    (key) => selectedFiles[key as Id<"_storage">],
  ) as Id<"_storage">[];

  const [isDraggingFile, setIsDraggingFile] = useState(false);
  const useUploadFilesResult = useUploadFiles();
  const containerRef = useRef<HTMLDivElement>(null);

  const totalNumFiles = useQuery(udfs.fileStorageV2.numFiles, {
    componentId: useNents().selectedNent?.id ?? null,
  });

  return (
    <PageContent>
      <DeploymentPageTitle title="Files" />
      <div
        className="relative flex h-full min-w-[36.25rem] flex-col gap-4 p-6 py-4 scrollbar"
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
            totalNumFiles={totalNumFiles}
          />
        </div>
        <FilesList
          selectedFiles={selectedFiles}
          setSelectedFiles={setSelectedFiles}
          containerRef={containerRef}
          totalNumFiles={totalNumFiles}
        />
        {isDraggingFile && (
          // eslint-disable-next-line no-restricted-syntax
          <div className="pointer-events-none absolute inset-0 z-50 mx-6 my-4 flex max-w-[60rem] animate-fadeInFromLoading items-center justify-center rounded-lg border-2 border-dashed bg-background-secondary/70 text-center text-lg tracking-tight text-content-tertiary backdrop-blur-sm">
            <UploadIcon className="mr-2 size-6" />
            Drop files to upload
          </div>
        )}
      </div>
    </PageContent>
  );
}
