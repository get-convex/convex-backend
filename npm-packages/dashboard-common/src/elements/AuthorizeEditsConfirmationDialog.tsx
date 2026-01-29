import { ConfirmationDialog } from "@ui/ConfirmationDialog";

export function AuthorizeEditsConfirmationDialog({
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
            {/* TODO(ENG-10340) Remove prod-specific messaging and use the deployment ref instead */}
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
