import { UploadIcon } from "@radix-ui/react-icons";
import { useMutation } from "convex/react";
import { useContext, useRef, useState } from "react";
import udfs from "@common/udfs";
import { toast } from "@common/lib/utils";
import { useNents } from "@common/lib/useNents";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { buttonClasses } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Spinner } from "@ui/Spinner";

const isHtmlContent = (file: File): boolean =>
  file.type.includes("html") ||
  file.type === "text/html" ||
  file.name.endsWith(".html") ||
  file.name.endsWith(".htm");

const checkFileForHtmlContent = (file: File): Promise<boolean> =>
  new Promise((resolve) => {
    // Only check first 4KB of the file
    const chunk = file.slice(0, 4096);
    const reader = new FileReader();

    reader.onload = (e) => {
      const content = e.target?.result as string;
      const contentLower = content.toLowerCase();
      // Check for common HTML patterns
      const hasHtmlPatterns =
        /<\s*(!doctype|html|head|body|script|div|a|meta)[^>]*>/i.test(
          contentLower,
        );
      resolve(hasHtmlPatterns);
    };

    reader.onerror = () => resolve(false);
    reader.readAsText(chunk);
  });

export function useUploadFiles() {
  const {
    useCurrentDeployment,
    useHasProjectAdminPermissions,
    captureException,
  } = useContext(DeploymentInfoContext);
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
      try {
        if (isHtmlContent(file) || (await checkFileForHtmlContent(file))) {
          captureException(
            new Error(`Uploaded file appears to be HTML content.`),
          );
        }
      } catch (error) {
        captureException(error);
      }

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
            ? `File "${successes[0].name}"`
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
            ? `file "${failures[0].name}"`
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
    <div className="flex items-center justify-center gap-2">
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
