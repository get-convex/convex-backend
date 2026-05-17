import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TeamSettingsLayout } from "../../../../components/TeamSettingsLayout";
import { listTeams, Team } from "../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../lib/config";

export default function TeamGeneralPage() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const { data: teams, mutate } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team: Team | undefined = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteError, setDeleteError] = useState<string | undefined>();

  useEffect(() => {
    if (team) {
      setName(team.name);
      setSlug(team.slug);
    }
    // We intentionally only re-seed when the team identity changes; resetting
    // on every `team.name`/`team.slug` change would clobber unsaved edits.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [team?.id]);

  if (!mounted || !team || !token) return null;

  const dirty = name !== team.name || slug !== team.slug;

  const onSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);
    try {
      const res = await fetch(`${url}/api/dashboard/teams/${team.id}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ name, slug }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutate();
      if (slug !== team.slug) {
        await router.replace(`/t/${slug}/settings`);
      }
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const onConfirmDelete = async () => {
    setDeleteError(undefined);
    try {
      const res = await fetch(`${url}/api/dashboard/teams/${team.id}/delete`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutate();
      void router.replace("/");
    } catch (err) {
      setDeleteError((err as Error).message);
      throw err;
    }
  };

  return (
    <TeamSettingsLayout page="general" title="Team Settings">
      <Sheet>
        <h3>Edit Team</h3>
        <form onSubmit={onSave} className="mt-4 flex flex-col gap-4">
          <TextInput
            id="teamName"
            label="Team Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <TextInput
            id="teamSlug"
            label="Team Slug"
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            description="The slug appears in URLs. Changing it will invalidate any saved links."
          />
          {error && (
            <div className="text-xs text-content-error" role="alert">
              {error}
            </div>
          )}
          <div className="ml-auto">
            <Button type="submit" size="xs" disabled={!dirty || saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </form>
      </Sheet>
      <Sheet>
        <h3>Delete Team</h3>
        <p className="mt-2 text-sm text-content-secondary">
          Permanently deletes this team. To delete your team, you must first
          remove all team members and delete all projects associated with the
          team.
        </p>
        <div className="mt-3">
          <Button
            variant="danger"
            size="xs"
            onClick={() => setDeleteOpen(true)}
          >
            Delete Team
          </Button>
        </div>
      </Sheet>
      {deleteOpen && (
        <ConfirmationDialog
          onClose={() => setDeleteOpen(false)}
          onConfirm={onConfirmDelete}
          confirmText="Delete Team"
          dialogTitle="Delete Team"
          error={deleteError}
          validationText={`Delete ${team.name} and all of its projects and deployments`}
          dialogBody={
            <>
              Delete this team along with every project, deployment, and member.
              <div className="mt-2 font-semibold">
                Deleted teams cannot be recovered.
              </div>
            </>
          }
        />
      )}
    </TeamSettingsLayout>
  );
}
