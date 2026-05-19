import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { PauseDeployment } from "@common/features/settings/components/PauseDeployment";
import { DeploymentSummary } from "@common/features/health/components/DeploymentSummary";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TrashIcon } from "@radix-ui/react-icons";
import { useRouter } from "next/router";
import { useContext, useRef, useState } from "react";
import { useSWRConfig } from "swr";
import { useScrollToHash } from "@common/lib/useScrollToHash";
import { useAccessToken } from "../../../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../../../lib/config";

export default function DeploymentSettings() {
  const pauseRef = useRef<HTMLDivElement | null>(null);
  useScrollToHash("#pause-deployment", pauseRef);
  const { useCurrentTeam, useCurrentProject, useCurrentDeployment } =
    useContext(DeploymentInfoContext);
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();
  return (
    <DeploymentSettingsLayout page="general">
      <div className="flex flex-col gap-4">
        {deployment && team?.slug && project?.slug && (
          <DeploymentSummary
            deployment={deployment}
            teamSlug={team.slug}
            projectSlug={project.slug}
          />
        )}
        <Sheet>
          <h3>Deployment</h3>
          <p className="mt-2 max-w-prose text-content-secondary">
            Use the project-level Settings page on the orchestrator to manage
            deploy keys for this deployment.
          </p>
        </Sheet>
        <div ref={pauseRef}>
          <PauseDeployment />
        </div>
        <DeleteDeploymentSection />
      </div>
    </DeploymentSettingsLayout>
  );
}

function DeleteDeploymentSection() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
  const deploymentName = router.query.deploymentName as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { mutate } = useSWRConfig();

  const [showConfirm, setShowConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onDelete = async () => {
    if (!deploymentName || !token) return;
    setError(null);
    try {
      const res = await fetch(
        `${url}/v1/deployments/${encodeURIComponent(deploymentName)}/delete`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${token}` },
        },
      );
      if (!res.ok) {
        const body = await res.text().catch(() => "");
        throw new Error(`HTTP ${res.status}: ${body}`);
      }
      // Invalidate cached deployment lists so the deleted row disappears
      // from the project's deployment dropdown / index without a manual
      // page refresh. Project- and team-scoped lists both use a tuple key
      // starting with "deployments"; revalidate them all.
      await mutate(
        (key) => Array.isArray(key) && key[0] === "deployments",
        undefined,
        { revalidate: true },
      );
      // Drop the localStorage "last viewed" pointer so the project
      // redirector doesn't try to bounce us back into the deleted row.
      if (typeof window !== "undefined" && projectSlug) {
        try {
          window.localStorage.removeItem(`orch-last-deployment-${projectSlug}`);
        } catch {
          // localStorage may throw in strict-tracking-protected browsers;
          // a stale pointer is harmless because `pickDefault` falls back
          // when the name doesn't match an existing deployment.
        }
      }
      void router.replace(`/t/${teamSlug}/${projectSlug}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <Sheet>
      <h3>Delete Deployment</h3>
      <p className="mt-2 max-w-prose text-sm text-content-secondary">
        Permanently delete{" "}
        <code className="rounded-sm bg-background-tertiary px-1 font-mono">
          {deploymentName}
        </code>
        . The orchestrator will tear down the backend and remove all deploy
        keys, env vars, and audit-log entries scoped to this deployment.
        Snapshots stored on the deployment&apos;s backend storage are deleted
        with it. This cannot be undone.
      </p>
      {error && (
        <Callout variant="error" className="mt-3">
          {error}
        </Callout>
      )}
      <div className="mt-4">
        <Button
          variant="danger"
          icon={<TrashIcon />}
          onClick={() => setShowConfirm(true)}
        >
          Delete Deployment
        </Button>
      </div>
      {showConfirm && deploymentName && (
        <ConfirmationDialog
          dialogTitle="Delete deployment"
          dialogBody={
            <>
              <p className="text-sm">
                The deployment{" "}
                <span className="font-semibold">{deploymentName}</span> and all
                of its data will be permanently destroyed.
              </p>
              <p className="mt-3 text-sm text-content-secondary">
                Type <code className="font-mono">{deploymentName}</code> to
                confirm.
              </p>
            </>
          }
          validationText={deploymentName}
          confirmText="Delete deployment"
          onClose={() => setShowConfirm(false)}
          onConfirm={onDelete}
        />
      )}
    </Sheet>
  );
}
