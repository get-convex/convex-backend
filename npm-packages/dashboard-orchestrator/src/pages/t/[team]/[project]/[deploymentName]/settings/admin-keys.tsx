// Admin keys are owned by the running backend, not the orchestrator. The
// orchestrator's role here is just to surface the backend's `/api/admin_keys`
// UI on a multi-deployment chrome — mirroring how the self-hosted dashboard
// already exposes per-deployment admin keys. Users paste these directly into
// `CONVEX_SELF_HOSTED_ADMIN_KEY` (and pair with `CONVEX_SELF_HOSTED_URL`) to
// drive `npx convex deploy` against this deployment.

import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { LoadingTransition } from "@ui/Loading";
import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { useContext, useState } from "react";

import { AdminKeysList } from "../../../../../../components/adminKeys/AdminKeysList";
import { CreateAdminKeyModal } from "../../../../../../components/adminKeys/CreateAdminKeyModal";
import { useAdminKeys } from "../../../../../../hooks/useAdminKeys";

export default function AdminKeysPage() {
  const ctx = useContext(DeploymentInfoContext);
  const [showCreate, setShowCreate] = useState(false);

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
        Convex CLI. Paste them into{" "}
        <code className="rounded-sm bg-background-tertiary px-1 text-xs">
          CONVEX_SELF_HOSTED_ADMIN_KEY
        </code>{" "}
        (with{" "}
        <code className="rounded-sm bg-background-tertiary px-1 text-xs">
          CONVEX_SELF_HOSTED_URL={deploymentUrl}
        </code>
        ) for{" "}
        <code className="rounded-sm bg-background-tertiary px-1 text-xs">
          npx convex deploy
        </code>{" "}
        in CI or hosting integrations.
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
            onRevoke={async (id) => {
              await revoke(id);
              // Unlike the self-hosted dashboard, we don't need to clear
              // session credentials when the current key is revoked — the
              // orchestrator's `OrchestratorDeploymentShell` mints a fresh
              // bootstrap key on every shell load via `fetchDeploymentAuth`,
              // so a hard reload is enough.
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
