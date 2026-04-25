import { Button } from "@ui/Button";
import { Modal } from "@ui/Modal";
import { useState } from "react";

export function RevokeAdminKeyButton({
  id,
  isCurrent,
  onRevoke,
}: {
  id: string;
  isCurrent: boolean;
  onRevoke: (id: string, isCurrent: boolean) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);

  return (
    <>
      <Button variant="neutral" size="xs" onClick={() => setOpen(true)}>
        Revoke
      </Button>
      {open && (
        <Modal title="Revoke Admin Key" onClose={() => setOpen(false)}>
          <p className="text-sm">
            {isCurrent
              ? "Revoking this key will immediately log you out of the dashboard. You'll need another admin key, or to run generate_admin_key.sh again, to log back in."
              : "This action can't be undone. Applications using this key will stop working."}
          </p>
          <div className="mt-4 flex justify-end gap-2">
            <Button variant="neutral" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              loading={loading}
              onClick={async () => {
                setLoading(true);
                try {
                  await onRevoke(id, isCurrent);
                } finally {
                  setLoading(false);
                  setOpen(false);
                }
              }}
            >
              Revoke
            </Button>
          </div>
        </Modal>
      )}
    </>
  );
}
