import { Button } from "@ui/Button";
import { Modal } from "@ui/Modal";
import { TextInput } from "@ui/TextInput";
import { useState } from "react";

export function RenameAdminKeyButton({
  id,
  name,
  onRename,
}: {
  id: string;
  name: string;
  onRename: (id: string, name: string) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState(name);
  const [loading, setLoading] = useState(false);

  return (
    <>
      <Button
        variant="neutral"
        size="xs"
        onClick={() => {
          setDraft(name);
          setOpen(true);
        }}
      >
        Rename
      </Button>
      {open && (
        <Modal title="Rename Admin Key" onClose={() => setOpen(false)}>
          <form
            className="flex flex-col gap-3"
            onSubmit={async (e) => {
              e.preventDefault();
              if (!draft.trim()) return;
              setLoading(true);
              try {
                await onRename(id, draft.trim());
                setOpen(false);
              } finally {
                setLoading(false);
              }
            }}
          >
            <TextInput
              id="name"
              label="Name"
              autoFocus
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <div className="flex justify-end gap-2">
              <Button variant="neutral" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={!draft.trim()} loading={loading}>
                Save
              </Button>
            </div>
          </form>
        </Modal>
      )}
    </>
  );
}
