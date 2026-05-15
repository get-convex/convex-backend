import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { Modal } from "@ui/Modal";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { CopyButton } from "@common/elements/CopyButton";
import { TeamSettingsLayout } from "../../../../components/TeamSettingsLayout";
import { listTeams, Team } from "../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../lib/config";

type AccessToken = {
  id: string;
  kind: string;
  name: string;
  creationTime: number;
  keySuffix: string;
};

export default function AccessTokensPage() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const { data: teams } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team: Team | undefined = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );

  const { data: tokens, mutate } = useSWR<AccessToken[]>(
    team && token ? ["accessTokens", team.id, token] : null,
    () =>
      fetchJson<AccessToken[]>(
        `${url}/api/dashboard/teams/${team!.id}/access_tokens`,
        token!,
      ),
  );

  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [revoking, setRevoking] = useState<AccessToken | null>(null);
  const [revokeError, setRevokeError] = useState<string | undefined>();

  if (!mounted || !team || !token) return null;

  const onCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const res = await fetch(`${url}/v1/create_personal_access_token`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ name: newName }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const body = (await res.json()) as { accessToken: string };
      setCreatedToken(body.accessToken);
      setNewName("");
      await mutate();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  const onConfirmRevoke = async () => {
    if (!revoking) return;
    setRevokeError(undefined);
    try {
      const res = await fetch(`${url}/v1/delete_personal_access_token`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ id: revoking.id }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutate();
    } catch (err) {
      setRevokeError((err as Error).message);
      throw err;
    }
  };

  const teamId = team.id;
  const myTokens = (tokens ?? []).filter((t) => t.kind === "pat");

  return (
    <TeamSettingsLayout page="access-tokens" title="Team Access Tokens">
      <Sheet>
        <div className="flex items-start justify-between gap-3">
          <div className="flex flex-col gap-3 text-sm">
            <p className="text-content-primary">
              These access tokens allow your team to access this orchestrator's
              management API.
            </p>
            <div>
              <div className="font-semibold text-content-primary">Team ID</div>
              <code className="font-mono text-sm text-content-primary">
                {teamId}
              </code>
            </div>
            <div>
              <div className="font-semibold text-content-primary">
                What can team access tokens do?
              </div>
              <ul className="mt-1 list-disc pl-5 text-content-primary">
                <li>Create new projects</li>
                <li>Create new deployments</li>
                <li>Manage all projects on the team</li>
                <li>Read and write data in all projects</li>
              </ul>
            </div>
            <p className="text-content-primary">
              You cannot see tokens that other members of your team have
              created.
            </p>
          </div>
          <Button size="xs" onClick={() => setShowCreate(true)}>
            + Create Token
          </Button>
        </div>
        {error && (
          <div className="mt-2 text-xs text-content-error" role="alert">
            {error}
          </div>
        )}
        <ul className="mt-4 divide-y divide-border-transparent">
          {myTokens.map((t) => (
            <li
              key={t.id}
              className="flex items-center justify-between gap-3 py-3"
            >
              <div>
                <div className="text-sm font-medium text-content-primary">
                  {t.name}
                </div>
                <div className="font-mono text-xs text-content-secondary">
                  pat:…{t.keySuffix}
                </div>
              </div>
              <Button size="xs" variant="danger" onClick={() => setRevoking(t)}>
                Revoke
              </Button>
            </li>
          ))}
          {myTokens.length === 0 && (
            <li className="py-3 text-sm text-content-secondary">
              You have not created any team access tokens yet.
            </li>
          )}
        </ul>
      </Sheet>

      {showCreate && (
        <Modal title="Create access token" onClose={() => setShowCreate(false)}>
          <form onSubmit={onCreate} className="flex flex-col gap-4">
            <TextInput
              id="tokenName"
              label="Name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. CI deploy key"
              autoFocus
            />
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="neutral"
                size="xs"
                onClick={() => setShowCreate(false)}
              >
                Cancel
              </Button>
              <Button type="submit" size="xs" disabled={!newName || submitting}>
                {submitting ? "Creating…" : "Create"}
              </Button>
            </div>
          </form>
        </Modal>
      )}

      {revoking && (
        <ConfirmationDialog
          dialogTitle="Revoke access token"
          confirmText="Revoke token"
          onClose={() => setRevoking(null)}
          onConfirm={onConfirmRevoke}
          error={revokeError}
          dialogBody={
            <>
              Revoke <span className="font-semibold">{revoking.name}</span>.
              Anything using this token will stop working immediately.
            </>
          }
        />
      )}

      {createdToken && (
        <Modal title="Token created" onClose={() => setCreatedToken(null)}>
          <p className="mb-3 text-sm text-content-secondary">
            Copy this token now — you won't see it again.
          </p>
          <div className="flex items-center gap-2">
            <code className="flex-1 truncate rounded-sm bg-background-tertiary p-2 font-mono text-xs">
              {createdToken}
            </code>
            <CopyButton text={createdToken} />
          </div>
          <div className="mt-4 flex justify-end">
            <Button size="xs" onClick={() => setCreatedToken(null)}>
              Done
            </Button>
          </div>
        </Modal>
      )}
    </TeamSettingsLayout>
  );
}

async function fetchJson<T>(u: string, token: string): Promise<T> {
  const res = await fetch(u, { headers: { Authorization: `Bearer ${token}` } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return (await res.json()) as T;
}
