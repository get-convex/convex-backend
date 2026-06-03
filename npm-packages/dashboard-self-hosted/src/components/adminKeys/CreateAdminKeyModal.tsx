import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Modal } from "@ui/Modal";
import { TextInput } from "@ui/TextInput";
import { CopyButton } from "@common/elements/CopyButton";
import { useState } from "react";

import type { CreatedAdminKey } from "../../hooks/useAdminKeys";

export function CreateAdminKeyModal({
  onCreate,
  onClose,
}: {
  onCreate: (name: string) => Promise<CreatedAdminKey>;
  onClose: () => void;
}) {
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [created, setCreated] = useState<CreatedAdminKey | null>(null);

  return (
    <Modal
      title={created ? "Admin Key Created" : "Create Admin Key"}
      onClose={onClose}
    >
      {created ? (
        <div className="flex flex-col gap-4">
          <p className="text-sm">
            Copy your new admin key now. You won&apos;t be able to see it again.
          </p>
          <div className="flex items-center gap-2">
            <code className="min-w-0 flex-1 truncate rounded-sm bg-background-tertiary px-2 py-1 text-sm">
              {created.adminKey}
            </code>
            <CopyButton text={created.adminKey} />
          </div>
          <div className="flex justify-end">
            <Button onClick={onClose}>Done</Button>
          </div>
        </div>
      ) : (
        <form
          className="flex flex-col gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            if (!name.trim()) return;
            setLoading(true);
            try {
              const res = await onCreate(name.trim());
              setCreated(res);
            } finally {
              setLoading(false);
            }
          }}
        >
          <TextInput
            id="name"
            label="Name"
            autoFocus
            value={name}
            placeholder="Enter a memorable name for your admin key"
            onChange={(e) => setName(e.target.value)}
          />
          <div className="flex justify-end">
            <Button
              type="submit"
              icon={<PlusIcon />}
              disabled={!name.trim()}
              loading={loading}
            >
              Create
            </Button>
          </div>
        </form>
      )}
    </Modal>
  );
}
