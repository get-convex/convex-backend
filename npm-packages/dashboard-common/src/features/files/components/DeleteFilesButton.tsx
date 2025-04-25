import { TrashIcon } from "@radix-ui/react-icons";
import { useState, useContext } from "react";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { Button } from "@ui/Button";
import { useNents } from "@common/lib/useNents";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { DeleteFileModal } from "./DeleteFileModal";

export function DeleteFilesButton({
  selectedFiles,
}: {
  selectedFiles: Id<"_storage">[];
}) {
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );
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
