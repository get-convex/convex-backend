import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { Menu, MenuItem } from "@ui/Menu";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { CaretSortIcon, CheckIcon } from "@radix-ui/react-icons";
import { TeamSettingsLayout } from "../../../../components/TeamSettingsLayout";
import { listTeams, Team } from "../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../lib/useOrchestratorToken";
import { useSession } from "../../../../lib/auth-client";
import { orchestratorUrl } from "../../../../lib/config";

type Member = { id: number; email: string; name: string | null; role: string };
type Invitation = {
  id: number;
  email: string;
  role: string;
  code: string;
  createdAt: number;
};

export default function TeamMembersPage() {
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

  const { data: members, mutate: mutateMembers } = useSWR<Member[]>(
    team && token ? ["members", team.id, token] : null,
    () => fetchJson(`${url}/api/dashboard/teams/${team!.id}/members`, token!),
  );
  const { data: invites, mutate: mutateInvites } = useSWR<Invitation[]>(
    team && token ? ["invites", team.id, token] : null,
    () => fetchJson(`${url}/api/dashboard/teams/${team!.id}/invites`, token!),
  );

  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState<"developer" | "admin">(
    "developer",
  );
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [removing, setRemoving] = useState<Member | null>(null);
  const [removeError, setRemoveError] = useState<string | undefined>();

  const session = useSession();
  const myEmail = session?.data?.user?.email ?? "";
  const adminCount = (members ?? []).filter((m) => m.role === "admin").length;

  if (!mounted || !team || !token) return null;

  const onInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const res = await fetch(`${url}/api/dashboard/teams/${team.id}/invites`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ email: inviteEmail, role: inviteRole }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setInviteEmail("");
      await mutateInvites();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  const onRoleChange = async (memberId: number, role: string) => {
    setError(null);
    try {
      const res = await fetch(
        `${url}/api/dashboard/teams/${team.id}/update_member_role`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ memberId, role }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutateMembers();
    } catch (err) {
      setError((err as Error).message);
    }
  };

  const onConfirmRemove = async () => {
    if (!removing) return;
    setRemoveError(undefined);
    try {
      const res = await fetch(
        `${url}/api/dashboard/teams/${team.id}/remove_member`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ memberId: removing.id }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutateMembers();
    } catch (err) {
      setRemoveError((err as Error).message);
      throw err;
    }
  };

  const filtered = (members ?? []).filter(
    (m) =>
      !search ||
      m.email.toLowerCase().includes(search.toLowerCase()) ||
      (m.name ?? "").toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <TeamSettingsLayout page="members" title="Members">
      <Sheet>
        <h3>Invite Member</h3>
        <form onSubmit={onInvite} className="mt-4 flex items-end gap-2">
          <div className="flex-1">
            <TextInput
              id="inviteEmail"
              label="Email address"
              type="email"
              value={inviteEmail}
              onChange={(e) => setInviteEmail(e.target.value)}
              placeholder="member@example.com"
            />
          </div>
          <select
            value={inviteRole}
            onChange={(e) =>
              setInviteRole(e.target.value as "developer" | "admin")
            }
            className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            aria-label="Role"
          >
            <option value="developer">Developer</option>
            <option value="admin">Admin</option>
          </select>
          <Button type="submit" size="xs" disabled={!inviteEmail || submitting}>
            Send Invite
          </Button>
        </form>
        {error && (
          <div className="mt-2 text-xs text-content-error" role="alert">
            {error}
          </div>
        )}
      </Sheet>

      <Sheet>
        <div className="flex items-center justify-between gap-3">
          <h3>Team Members</h3>
          <div className="w-64">
            <TextInput
              id="searchMembers"
              type="search"
              label="Search members"
              labelHidden
              placeholder="Search members"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
        </div>
        <ul className="mt-4 divide-y divide-border-transparent">
          {filtered.map((m) => {
            const isSelf = m.email === myEmail;
            const isLastAdmin = m.role === "admin" && adminCount <= 1;
            return (
              <li
                key={m.id}
                className="flex items-center justify-between gap-3 py-3"
              >
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium text-content-primary">
                    {m.name ?? m.email.split("@")[0]}
                  </div>
                  <div className="truncate text-xs text-content-secondary">
                    {m.email}
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <RoleMenu
                    role={m.role}
                    disabled={isLastAdmin}
                    onChange={(role) => onRoleChange(m.id, role)}
                  />
                  <Button
                    size="xs"
                    variant="danger"
                    disabled={isLastAdmin && isSelf}
                    tip={
                      isLastAdmin && isSelf
                        ? "You're the last admin — promote someone else first."
                        : undefined
                    }
                    onClick={() => setRemoving(m)}
                  >
                    {isSelf ? "Leave team" : "Remove"}
                  </Button>
                </div>
              </li>
            );
          })}
          {filtered.length === 0 && (
            <li className="py-3 text-sm text-content-secondary">No members.</li>
          )}
        </ul>
      </Sheet>

      {invites && invites.length > 0 && (
        <Sheet>
          <h3>Pending Invitations</h3>
          <ul className="mt-4 divide-y divide-border-transparent">
            {invites.map((inv) => (
              <li
                key={inv.id}
                className="flex items-center justify-between gap-3 py-2"
              >
                <div>
                  <div className="text-sm font-medium text-content-primary">
                    {inv.email}
                  </div>
                  <div className="text-xs text-content-secondary">
                    {inv.role} · code {inv.code.slice(0, 8)}…
                  </div>
                </div>
              </li>
            ))}
          </ul>
        </Sheet>
      )}
      {removing && (
        <ConfirmationDialog
          dialogTitle={
            removing.email === myEmail ? "Leave team" : "Remove member"
          }
          confirmText={
            removing.email === myEmail ? "Leave team" : "Remove member"
          }
          onClose={() => setRemoving(null)}
          onConfirm={onConfirmRemove}
          error={removeError}
          dialogBody={
            removing.email === myEmail ? (
              <>
                Leave <span className="font-semibold">{team.name}</span>. You
                will lose access to all of the team's projects and deployments
                unless another member re-invites you.
              </>
            ) : (
              <>
                Remove{" "}
                <span className="font-semibold">
                  {removing.name ?? removing.email}
                </span>{" "}
                from <span className="font-semibold">{team.name}</span>. They
                will lose access to all of the team's projects.
              </>
            )
          }
        />
      )}
    </TeamSettingsLayout>
  );
}

function RoleMenu({
  role,
  disabled,
  onChange,
}: {
  role: string;
  disabled: boolean;
  onChange: (role: string) => void;
}) {
  const display = role === "admin" ? "Admin" : "Developer";
  if (disabled) {
    return <span className="text-sm text-content-secondary">{display}</span>;
  }
  return (
    <Menu
      buttonProps={{
        variant: "unstyled",
        "aria-label": "Change role",
        children: (
          <span className="inline-flex items-center gap-1 rounded-sm px-1.5 py-1 text-sm text-content-secondary hover:bg-background-tertiary">
            {display}
            <CaretSortIcon className="size-3" />
          </span>
        ),
      }}
      placement="bottom-end"
    >
      <MenuItem action={() => onChange("admin")}>
        <span className="flex w-full items-center justify-between">
          Admin {role === "admin" && <CheckIcon className="size-3.5" />}
        </span>
      </MenuItem>
      <MenuItem action={() => onChange("developer")}>
        <span className="flex w-full items-center justify-between">
          Developer {role === "developer" && <CheckIcon className="size-3.5" />}
        </span>
      </MenuItem>
    </Menu>
  );
}

async function fetchJson<T>(u: string, token: string): Promise<T> {
  const res = await fetch(u, { headers: { Authorization: `Bearer ${token}` } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return (await res.json()) as T;
}
