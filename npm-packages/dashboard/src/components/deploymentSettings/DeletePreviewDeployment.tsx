import { TrashIcon } from "@radix-ui/react-icons";
import { useCurrentDeployment } from "api/deployments";
import { Button } from "dashboard-common/elements/Button";
import { TimestampDistance } from "dashboard-common/elements/TimestampDistance";
import { Sheet } from "dashboard-common/elements/Sheet";
import { ConfirmationDialog } from "dashboard-common/elements/ConfirmationDialog";
import { useDeletePreviewDeployment } from "hooks/api";
import { useRouter } from "next/router";
import { useState } from "react";

export function DeletePreviewDeployment() {
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const deployment = useCurrentDeployment();
  if (deployment === undefined || !deployment.previewIdentifier) {
    return null;
  }
  return (
    <Sheet>
      <div className="mb-4 flex flex-row items-baseline gap-4">
        <h3 className="break-all">
          Delete deployment for <code>{deployment.previewIdentifier}</code>
        </h3>
        <TimestampDistance
          prefix="Created "
          date={new Date(deployment.createTime)}
        />
      </div>

      <p className="mb-5 text-sm text-content-primary">
        This deployment will be permanently deleted. This action cannot be
        undone.
      </p>
      <Button
        variant="danger"
        onClick={() => setShowDeleteModal(!showDeleteModal)}
        className="float-right"
        icon={<TrashIcon />}
      >
        Delete
      </Button>

      {showDeleteModal && (
        <DeletePreviewDeploymentModal
          projectId={deployment.projectId}
          identifier={deployment.previewIdentifier}
          onClose={() => setShowDeleteModal(false)}
        />
      )}
    </Sheet>
  );
}

function DeletePreviewDeploymentModal({
  identifier,
  projectId,
  onClose,
}: {
  identifier: string;
  projectId: number;
  onClose: () => void;
}) {
  const router = useRouter();
  const deletePreviewDeployment = useDeletePreviewDeployment(projectId);
  const handleDelete = async () => {
    await deletePreviewDeployment({
      identifier,
    });
    const teamSlug = router.query.team as string;
    const projectSlug = router.query.project as string;
    const projectURI = `/t/${teamSlug}/${projectSlug}`;
    await router.push(projectURI);
  };

  return (
    <ConfirmationDialog
      onClose={onClose}
      onConfirm={handleDelete}
      confirmText="Delete"
      dialogTitle="Delete Preview Deployment"
      dialogBody={
        <>
          Are you sure you want to delete the preview deployment for{" "}
          <code>{identifier}</code>? This cannot be undone.
        </>
      }
    />
  );
}
