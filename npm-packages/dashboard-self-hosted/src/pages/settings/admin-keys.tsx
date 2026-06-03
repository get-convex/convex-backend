import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { LoadingTransition } from "@ui/Loading";
import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
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
        <Sheet>
          <div className="text-content-secondary">
            Loading deployment credentials…
          </div>
        </Sheet>
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
    <Sheet>
      <div className="mb-2 flex w-full items-center justify-between">
        <h3>Admin Keys</h3>
        <Button
          icon={<PlusIcon />}
          onClick={() => setShowCreate(true)}
          disabled={error !== undefined}
        >
          Create Admin Key
        </Button>
      </div>
      <p className="mb-2 max-w-prose text-content-primary">
        Admin keys grant full access to this deployment via the dashboard or the
        Convex CLI. Use them in CI environments, hosting integrations, or to log
        additional people into this dashboard. Any key first seen by the backend
        — including those minted by{" "}
        <code className="rounded-sm bg-background-tertiary px-1 text-xs">
          generate_admin_key.sh
        </code>{" "}
        — is automatically tracked here so you can revoke it later.
      </p>

      {error && (
        <div className="my-4 text-sm text-content-errorSecondary">
          Failed to load admin keys: {error.message}
        </div>
      )}

      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "h-14 w-full" }}
      >
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
      </LoadingTransition>

      {showCreate && (
        <CreateAdminKeyModal
          onCreate={create}
          onClose={() => setShowCreate(false)}
        />
      )}
    </Sheet>
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
