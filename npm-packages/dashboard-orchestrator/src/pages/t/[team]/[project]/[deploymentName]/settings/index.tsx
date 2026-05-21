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
import { useContext, useMemo, useRef, useState } from "react";
import { useSWRConfig } from "swr";
import { useScrollToHash } from "@common/lib/useScrollToHash";
import { useAccessToken } from "../../../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../../../lib/config";
import { useDeploymentSettings } from "../../../../../../hooks/useDeploymentSettings";
import { useProjectSettings } from "../../../../../../hooks/useProjectSettings";
import { useHostCapacity } from "../../../../../../hooks/useHostCapacity";
import { useKnobRegistry } from "../../../../../../hooks/useKnobRegistry";
import {
  BackendSettingsForm,
  type BackendSettingsDraft,
} from "../../../../../../components/backendSettings/BackendSettingsForm";

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
            // Orchestrator doesn't surface a "last backup time"; pass null so
            // the summary renders "No backup yet" rather than an infinite
            // spinner. The Backup & Restore page itself talks to the running
            // backend directly.
            lastBackupTime={null}
          />
        )}
        {deployment && (
          <DeploymentBackendSection deploymentName={deployment.name} />
        )}
        <div ref={pauseRef}>
          <PauseDeployment />
        </div>
        <DeleteDeploymentSection />
      </div>
    </DeploymentSettingsLayout>
  );
}

function DeploymentBackendSection({
  deploymentName,
}: {
  deploymentName: string;
}) {
  const { useCurrentProject } = useContext(DeploymentInfoContext);
  const project = useCurrentProject();
  const { settings, save, restart } = useDeploymentSettings(deploymentName);
  const { settings: projectSettings } = useProjectSettings(project?.id);
  const { data: capacity } = useHostCapacity();
  const { data: registry } = useKnobRegistry();

  const initialDraft = useMemo<BackendSettingsDraft>(
    () => ({
      tier: settings?.desiredTier ?? settings?.effectiveTier ?? "S16",
      overrides: settings?.desiredOverrides ?? {},
      force: false,
    }),
    // Re-seed only when remote settings load for the first time.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [settings?.desiredTier, settings?.effectiveTier],
  );

  const [draft, setDraft] = useState<BackendSettingsDraft>(initialDraft);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [showRestartConfirm, setShowRestartConfirm] = useState(false);
  const [restartError, setRestartError] = useState<string | null>(null);
  const [restarting, setRestarting] = useState(false);

  // Reset local draft whenever remote settings arrive (first load or after
  // a successful save).
  const prevEffective = useRef<string | undefined>(undefined);
  if (
    settings &&
    settings.effectiveTier !== prevEffective.current
  ) {
    prevEffective.current = settings.effectiveTier;
    setDraft({
      tier: settings.desiredTier ?? settings.effectiveTier,
      overrides: settings.desiredOverrides,
      force: false,
    });
  }

  const tierDefaults = useMemo<Record<string, string>>(() => ({}), []);

  const hasDrift =
    settings !== undefined &&
    (settings.runningTier !== settings.effectiveTier ||
      JSON.stringify(settings.runningOverrides) !==
        JSON.stringify({
          ...(projectSettings?.knobOverrides ?? {}),
          ...settings.desiredOverrides,
        }));

  const handleSave = async () => {
    if (!settings) return;
    setSaveError(null);
    setSaving(true);
    try {
      // If the user picked the same tier as the project, clear the
      // per-deployment override so the deployment inherits again.
      const projectTier = projectSettings?.tier ?? settings.effectiveTier;
      const desiredTier =
        draft.tier === projectTier ? null : draft.tier;

      // Build the overrides patch: send null for any key that was in the
      // original desired_overrides but is no longer in the draft.
      const patchOverrides: Record<string, string | null> = {
        ...draft.overrides,
      };
      for (const key of Object.keys(settings.desiredOverrides)) {
        if (!(key in draft.overrides)) {
          patchOverrides[key] = null;
        }
      }

      await save({ desiredTier, desiredOverrides: patchOverrides });
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleRestart = async () => {
    setRestartError(null);
    setRestarting(true);
    try {
      await restart(draft.force);
      setShowRestartConfirm(false);
    } catch (err) {
      setRestartError(err instanceof Error ? err.message : String(err));
    } finally {
      setRestarting(false);
    }
  };

  if (!settings) {
    return (
      <Sheet>
        <h3>Backend</h3>
        <p className="mt-2 text-sm text-content-secondary">
          Loading settings…
        </p>
      </Sheet>
    );
  }

  const projectTier = projectSettings?.tier ?? settings.effectiveTier;

  return (
    <Sheet>
      <h3>Backend</h3>
      <p className="mt-1 mb-4 text-sm text-content-secondary">
        Inherits from project:{" "}
        <code className="rounded-sm bg-background-tertiary px-1 font-mono">
          {projectTier}
        </code>
        . Picking the same tier as the project clears the per-deployment
        override.
      </p>

      <BackendSettingsForm
        registry={registry}
        capacity={capacity}
        tierDefaults={tierDefaults}
        initial={{
          tier: settings.desiredTier ?? settings.effectiveTier,
          overrides: settings.desiredOverrides,
          force: false,
        }}
        onChange={setDraft}
      />

      {hasDrift && (
        <Callout variant="instructions" className="mt-4">
          Saved changes haven&apos;t been applied yet. Click{" "}
          <strong>Apply changes (Restart)</strong> to spawn a new container with
          these settings (existing data is preserved on the volume).
        </Callout>
      )}

      {saveError && (
        <Callout variant="error" className="mt-3">
          {saveError}
        </Callout>
      )}

      <div className="mt-4 flex gap-2">
        <Button
          variant="neutral"
          size="xs"
          onClick={() => void handleSave()}
          disabled={saving}
        >
          {saving ? "Saving…" : "Save"}
        </Button>
        <Button
          variant="neutral"
          size="xs"
          onClick={() => setShowRestartConfirm(true)}
          disabled={restarting}
        >
          Apply changes (Restart)
        </Button>
      </div>

      {showRestartConfirm && (
        <ConfirmationDialog
          dialogTitle={`Restart ${deploymentName}?`}
          dialogBody={
            <div className="flex flex-col gap-3">
              <p className="text-sm">
                The container for{" "}
                <span className="font-semibold">{deploymentName}</span> will be
                recreated with the saved settings. Data on the volume persists.
                This causes brief downtime.
              </p>
              {restartError && (
                <Callout variant="error">{restartError}</Callout>
              )}
            </div>
          }
          confirmText="Restart deployment"
          onClose={() => {
            setShowRestartConfirm(false);
            setRestartError(null);
          }}
          onConfirm={handleRestart}
        />
      )}
    </Sheet>
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
