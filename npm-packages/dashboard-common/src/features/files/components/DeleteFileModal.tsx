import { useMutation } from "convex/react";
import { Id } from "system-udfs/convex/_generated/dataModel";
import udfs from "@common/udfs";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { useNents } from "@common/lib/useNents";

export function DeleteFileModal({
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
            Are you sure you want delete{" "}
            <span className="rounded-sm bg-background-tertiary p-1 font-mono text-sm text-content-secondary">
              {storageIds[0]}
            </span>
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
