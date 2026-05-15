import { useRouter } from "next/router";
import { useState } from "react";
import { useSWRConfig } from "swr";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Modal } from "@ui/Modal";
import { Callout } from "@ui/Callout";
import { createTeam } from "../lib/orchestratorApi";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorUrl } from "../lib/config";

export function CreateTeamModal({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated?: (teamSlug: string) => void;
}) {
  const router = useRouter();
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { mutate } = useSWRConfig();
  const [name, setName] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !token) return;
    setSubmitting(true);
    setError(null);
    try {
      const team = await createTeam(url, token, name.trim());
      // Refresh any SWR cache keyed on the team list so headers/pickers see
      // the new team without a hard reload.
      await mutate(
        (key) =>
          Array.isArray(key) && (key[0] === "teams" || key[0] === "team"),
      );
      onClose();
      if (onCreated) onCreated(team.slug);
      else void router.push(`/t/${team.slug}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal title="Create Team" onClose={onClose}>
      <form onSubmit={submit} className="flex flex-col gap-4">
        <p className="text-sm text-content-secondary">
          Collaborate with your team members by creating a Convex Team.
        </p>
        <TextInput
          id="teamName"
          label="Team name"
          labelHidden
          placeholder="Team name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          autoFocus
        />
        {error && <Callout variant="error">{error}</Callout>}
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
          <Button
            type="submit"
            size="xs"
            disabled={!name.trim() || submitting}
            loading={submitting}
          >
            Create
          </Button>
        </div>
      </form>
    </Modal>
  );
}
