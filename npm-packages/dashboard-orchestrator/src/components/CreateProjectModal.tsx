import { Modal } from "@ui/Modal";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useMemo, useState } from "react";
import { useRouter } from "next/router";
import { createProject, Team } from "../lib/orchestratorApi";
import { orchestratorUrl } from "../lib/config";
import { useAccessToken } from "../lib/useOrchestratorToken";
import {
  BackendSettingsForm,
  type BackendSettingsDraft,
} from "./backendSettings/BackendSettingsForm";
import {
  DEFAULT_INFRASTRUCTURE,
  infrastructureOverrides,
} from "./backendSettings/backendInfrastructure";
import { DEFAULT_TIER, tierDefaultsForName } from "./backendSettings/tiers";
import { useHostCapacity } from "../hooks/useHostCapacity";
import { useKnobRegistry } from "../hooks/useKnobRegistry";

export function CreateProjectModal({
  team,
  onClose,
  onCreated,
}: {
  team: Team;
  onClose: () => void;
  onCreated?: (slug: string) => void;
}) {
  const router = useRouter();
  const token = useAccessToken();
  const url = orchestratorUrl();
  const [name, setName] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { data: capacity } = useHostCapacity();
  const { data: registry } = useKnobRegistry();
  const [draft, setDraft] = useState<BackendSettingsDraft>({
    tier: DEFAULT_TIER,
    overrides: {},
    infrastructure: DEFAULT_INFRASTRUCTURE,
  });

  const tierDefaults = useMemo<Record<string, string>>(
    () => tierDefaultsForName(draft.tier),
    [draft.tier],
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token) return;
    setSubmitting(true);
    setError(null);
    try {
      const infrastructure = infrastructureOverrides(
        draft.infrastructure ?? DEFAULT_INFRASTRUCTURE,
      );
      const overrides = {
        ...draft.overrides,
        ...infrastructure.overrides,
      };
      const res = await createProject(
        url,
        token,
        team.slug,
        name,
        "prod",
        draft.tier,
        Object.keys(overrides).length > 0 ? overrides : undefined,
        infrastructure.provisioningMode,
      );
      onCreated?.(res.projectSlug);
      onClose();
      void router.push(`/t/${team.slug}/${res.projectSlug}`);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal title="Create project" onClose={onClose}>
      <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
        <TextInput
          id="newProjectName"
          label="Project name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="My App"
          autoFocus
          description="A new prod deployment will be provisioned automatically."
        />
        <BackendSettingsForm
          registry={registry}
          capacity={capacity}
          tierDefaults={tierDefaults}
          initial={draft}
          showInfrastructure
          onChange={setDraft}
        />
        {error && (
          <div className="text-xs text-content-error" role="alert">
            {error}
          </div>
        )}
        <div className="flex justify-end gap-2">
          <Button
            type="button"
            variant="neutral"
            size="xs"
            onClick={onClose}
            disabled={submitting}
          >
            Cancel
          </Button>
          <Button type="submit" disabled={!name || submitting} size="xs">
            {submitting ? "Creating…" : "Create project"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
