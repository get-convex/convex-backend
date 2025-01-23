import { ConfirmationDialog } from "./ConfirmationDialog";

export function ProductionEditsConfirmationDialog({
  onClose,
  onConfirm,
}: {
  onClose(): void;
  onConfirm(): Promise<void>;
}) {
  return (
    <ConfirmationDialog
      onClose={onClose}
      onConfirm={onConfirm}
      confirmText="Confirm"
      variant="primary"
      dialogTitle="Enable edit mode"
      dialogBody={
        <div className="flex flex-col gap-2">
          <p>
            You are about to start editing data in a production environment. If
            this is intentional, click "Confirm".
          </p>
          <p>
            Once confirmed, you will not be asked to confirm again for the
            remainder of your browser session.
          </p>
        </div>
      }
    />
  );
}
