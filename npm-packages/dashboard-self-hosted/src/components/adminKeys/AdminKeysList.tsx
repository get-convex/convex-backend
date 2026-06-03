import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Menu, MenuItem } from "@ui/Menu";
import { Modal } from "@ui/Modal";
import { TextInput } from "@ui/TextInput";
import { useState } from "react";

import type { AdminKey } from "../../hooks/useAdminKeys";

export function AdminKeysList({
  keys,
  onRevoke,
  onRename,
}: {
  keys: AdminKey[];
  onRevoke: (id: string, isCurrent: boolean) => Promise<void>;
  onRename: (id: string, name: string) => Promise<void>;
}) {
  const [showRevoked, setShowRevoked] = useState(false);
  const activeKeys = keys.filter((k) => k.revokedTime === null);
  const revokedCount = keys.length - activeKeys.length;
  const visibleKeys = showRevoked ? keys : activeKeys;

  return (
    // Wrap in a single block so LoadingTransition's surrounding flex row
    // doesn't lay these children out side-by-side. Without this wrapper,
    // the toggle button below would render as a sibling column to the right
    // of the list and stretch to match its height.
    <div className="flex w-full flex-col">
      {visibleKeys.length === 0 ? (
        <div className="my-6 flex w-full justify-center text-content-secondary">
          There are no admin keys yet.
        </div>
      ) : (
        <div className="flex w-full flex-col divide-y">
          {[...visibleKeys]
            .sort((a, b) => b.creationTime - a.creationTime)
            .map((adminKey) => (
              <AdminKeyListItem
                key={adminKey.id}
                adminKey={adminKey}
                onRevoke={onRevoke}
                onRename={onRename}
              />
            ))}
        </div>
      )}
      {revokedCount > 0 && (
        <div className="mt-3 flex w-full justify-end">
          <Button
            variant="unstyled"
            size="xs"
            onClick={() => setShowRevoked((v) => !v)}
            className="text-xs text-content-secondary underline-offset-2 hover:text-content-primary hover:underline"
          >
            {showRevoked
              ? "Hide revoked keys"
              : `Show revoked keys (${revokedCount})`}
          </Button>
        </div>
      )}
    </div>
  );
}

function AdminKeyListItem({
  adminKey,
  onRevoke,
  onRename,
}: {
  adminKey: AdminKey;
  onRevoke: (id: string, isCurrent: boolean) => Promise<void>;
  onRename: (id: string, name: string) => Promise<void>;
}) {
  const [showRevoke, setShowRevoke] = useState(false);
  const [showRename, setShowRename] = useState(false);
  const isRevoked = adminKey.revokedTime !== null;

  return (
    <div className="flex w-full flex-col">
      <div className="my-2 flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <span>{adminKey.name}</span>
            {adminKey.isCurrent && (
              <span className="rounded-sm bg-background-tertiary px-1.5 py-0.5 text-xs text-content-secondary">
                This key
              </span>
            )}
          </div>
          {adminKey.keySuffix && (
            <code
              aria-label={`Admin key ending in ${adminKey.keySuffix}`}
              title={`Admin key ending in ${adminKey.keySuffix}`}
              className="font-mono text-xs text-content-secondary"
            >
              ····{adminKey.keySuffix}
            </code>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex flex-col items-end">
            <TimestampDistance
              prefix="Created "
              date={new Date(adminKey.creationTime)}
            />
            {isRevoked && (
              <TimestampDistance
                prefix="Revoked "
                date={new Date(adminKey.revokedTime!)}
              />
            )}
          </div>
          {!isRevoked && (
            <Menu
              placement="bottom-end"
              buttonProps={{
                variant: "neutral",
                size: "xs",
                icon: <DotsVerticalIcon />,
                "aria-label": "Admin key options",
              }}
            >
              <MenuItem action={() => setShowRename(true)}>Rename</MenuItem>
              <MenuItem variant="danger" action={() => setShowRevoke(true)}>
                Revoke
              </MenuItem>
            </Menu>
          )}
        </div>
      </div>
      {showRevoke && (
        <ConfirmationDialog
          onClose={() => setShowRevoke(false)}
          onConfirm={() => onRevoke(adminKey.id, adminKey.isCurrent)}
          confirmText="Revoke"
          dialogTitle="Revoke Admin Key"
          dialogBody={
            <>
              {adminKey.isCurrent ? (
                <>
                  Revoking{" "}
                  <span className="font-semibold">{adminKey.name}</span>
                  {adminKey.keySuffix && (
                    <>
                      {" "}
                      (
                      <code className="font-mono text-xs">
                        ····{adminKey.keySuffix}
                      </code>
                      )
                    </>
                  )}{" "}
                  will immediately log you out of the dashboard. You&apos;ll
                  need another admin key, or to run{" "}
                  <code className="rounded-sm bg-background-tertiary px-1 text-xs">
                    generate_admin_key.sh
                  </code>{" "}
                  again, to log back in.
                </>
              ) : (
                <>
                  Are you sure you want to revoke{" "}
                  <span className="font-semibold">{adminKey.name}</span>
                  {adminKey.keySuffix && (
                    <>
                      {" "}
                      (
                      <code className="font-mono text-xs">
                        ····{adminKey.keySuffix}
                      </code>
                      )
                    </>
                  )}
                  ? Applications using this key will stop working.
                </>
              )}
            </>
          }
        />
      )}
      {showRename && (
        <RenameAdminKeyDialog
          initialName={adminKey.name}
          onClose={() => setShowRename(false)}
          onSave={(newName) => onRename(adminKey.id, newName)}
        />
      )}
    </div>
  );
}

function RenameAdminKeyDialog({
  initialName,
  onClose,
  onSave,
}: {
  initialName: string;
  onClose: () => void;
  onSave: (name: string) => Promise<void>;
}) {
  const [draft, setDraft] = useState(initialName);
  const [loading, setLoading] = useState(false);

  return (
    <Modal title="Rename Admin Key" onClose={onClose}>
      <form
        className="flex flex-col gap-3"
        onSubmit={async (e) => {
          e.preventDefault();
          if (!draft.trim() || draft.trim() === initialName) return;
          setLoading(true);
          try {
            await onSave(draft.trim());
            onClose();
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
          <Button variant="neutral" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            disabled={!draft.trim() || draft.trim() === initialName}
            loading={loading}
          >
            Save
          </Button>
        </div>
      </form>
    </Modal>
  );
}
