import { DownloadIcon, TrashIcon } from "@radix-ui/react-icons";
import { useContext, useState } from "react";
import Link from "next/link";
import { FileMetadata } from "system-udfs/convex/_system/frontend/fileStorageV2";
import { Button, buttonClasses } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useNents } from "@common/lib/useNents";
import { DeleteFileModal } from "./DeleteFileModal";
import { PreviewImage } from "./PreviewImage";

export function FileActions({ file }: { file: FileMetadata }) {
  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );
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
