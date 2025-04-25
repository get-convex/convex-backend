import { Id } from "system-udfs/convex/_generated/dataModel";
import { DeleteFilesButton } from "./DeleteFilesButton";
import { Uploader, useUploadFiles } from "./Uploader";

export function FileStorageHeader({
  selectedFiles,
  useUploadFilesResult,
  totalNumFiles,
}: {
  selectedFiles: Id<"_storage">[];
  useUploadFilesResult: ReturnType<typeof useUploadFiles>;
  totalNumFiles: number | undefined;
}) {
  return (
    <div className="flex w-full min-w-fit max-w-[60rem] flex-wrap items-center justify-between gap-2">
      <div className="flex items-center gap-4">
        <div className="flex flex-1 flex-col gap-1">
          <h3>File Storage</h3>
          <div
            className="flex items-center gap-1 text-xs text-content-secondary"
            data-testid="fileCount"
          >
            <span className="text-xs">Total Files</span>
            {totalNumFiles !== undefined && (
              <span className="font-semibold">
                {totalNumFiles.toLocaleString()}
              </span>
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
