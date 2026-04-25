import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { useContext, useState } from "react";

import { AdminKeysList } from "../../components/adminKeys/AdminKeysList";
import { CreateAdminKeyModal } from "../../components/adminKeys/CreateAdminKeyModal";
import { useAdminKeys } from "../../hooks/useAdminKeys";

export default function AdminKeysPage() {
  const ctx = useContext(DeploymentInfoContext);
  const [showCreate, setShowCreate] = useState(false);

  // Bail if credentials aren't ready yet (e.g. invalid context state).
  if (!ctx.ok) {
    return (
      <DeploymentSettingsLayout page="admin-keys">
        <div className="text-sm text-content-secondary">
          Loading deployment credentials…
        </div>
      </DeploymentSettingsLayout>
    );
  }

  return (
    <DeploymentSettingsLayout page="admin-keys">
      <AdminKeysPageBody
        deploymentUrl={ctx.deploymentUrl}
        adminKey={ctx.adminKey}
        showCreate={showCreate}
        setShowCreate={setShowCreate}
      />
    </DeploymentSettingsLayout>
  );
}

function AdminKeysPageBody({
  deploymentUrl,
  adminKey,
  showCreate,
  setShowCreate,
}: {
  deploymentUrl: string;
  adminKey: string;
  showCreate: boolean;
  setShowCreate: (v: boolean) => void;
}) {
  const { keys, error, create, revoke, rename } = useAdminKeys({
    deploymentUrl,
    adminKey,
  });

  return (
    <div className="flex flex-col gap-4">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h3>Admin Keys</h3>
          <p className="text-sm text-content-secondary">
            Create, view, and revoke admin keys for this deployment. Admin keys
            grant full access to the dashboard and CLI.
          </p>
        </div>
        <Button icon={<PlusIcon />} onClick={() => setShowCreate(true)}>
          Create
        </Button>
      </header>

      {error && (
        <div className="text-sm text-content-errorSecondary">
          Failed to load admin keys: {error.message}
        </div>
      )}

      {keys && (
        <AdminKeysList
          keys={keys}
          onRevoke={async (id, isCurrent) => {
            await revoke(id);
            if (isCurrent) {
              // Mirror the credential-clearing path used by `Header.onLogout`
              // in `_app.tsx` (it zeroes the same session-storage keys via
              // `useSessionStorage`). After clearing we reload so the
              // `DeploymentInfoProvider` falls back to the credentials form.
              clearCurrentCredentials();
            }
          }}
          onRename={rename}
        />
      )}

      {showCreate && (
        <CreateAdminKeyModal
          onCreate={create}
          onClose={() => setShowCreate(false)}
        />
      )}
    </div>
  );
}

function clearCurrentCredentials() {
  if (typeof window === "undefined") return;
  // These keys are written by `useSessionStorage` calls in
  // `dashboard-self-hosted/src/pages/_app.tsx` (see `DeploymentInfoProvider`
  // and the `Header.onLogout` handler).
  sessionStorage.removeItem("adminKey");
  sessionStorage.removeItem("deploymentUrl");
  sessionStorage.removeItem("deploymentName");
  window.location.reload();
}
