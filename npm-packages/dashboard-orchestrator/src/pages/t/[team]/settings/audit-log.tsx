import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import { Sheet } from "@ui/Sheet";
import { TeamSettingsLayout } from "../../../../components/TeamSettingsLayout";
import { listTeams, Team } from "../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../lib/config";

type AuditEvent = {
  id: number;
  teamId: number;
  memberId: number | null;
  action: string;
  metadata: Record<string, unknown>;
  creationTime: number;
};

type AuditPage = { events: AuditEvent[]; cursor: string | null };

type Row =
  | { kind: "single"; event: AuditEvent }
  | {
      kind: "group";
      action: string;
      actor: string;
      count: number;
      first: number;
      last: number;
      sample: AuditEvent;
    };

const FIVE_MIN = 5 * 60 * 1000;

function actorOf(e: AuditEvent): string {
  const m = e.metadata as { email?: string };
  return m?.email ?? `member ${e.memberId ?? "?"}`;
}

function groupNoise(events: AuditEvent[]): Row[] {
  const out: Row[] = [];
  for (const e of events) {
    const last = out[out.length - 1];
    if (
      last &&
      last.kind === "group" &&
      last.action === e.action &&
      last.actor === actorOf(e) &&
      e.creationTime - last.last <= FIVE_MIN
    ) {
      last.count += 1;
      last.last = e.creationTime;
      continue;
    }
    if (
      last &&
      last.kind === "single" &&
      last.event.action === e.action &&
      actorOf(last.event) === actorOf(e) &&
      e.creationTime - last.event.creationTime <= FIVE_MIN
    ) {
      out[out.length - 1] = {
        kind: "group",
        action: e.action,
        actor: actorOf(e),
        count: 2,
        first: last.event.creationTime,
        last: e.creationTime,
        sample: last.event,
      };
      continue;
    }
    out.push({ kind: "single", event: e });
  }
  return out;
}

function humanizeAction(s: string): string {
  return s
    .replace(/([A-Z])/g, " $1")
    .replace(/^./, (c) => c.toUpperCase())
    .trim();
}

type Member = { id: number; email: string; name: string | null; role: string };

export default function AuditLogPage() {
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

  // Filter state. memberId/action/from/to map to backend AuditFilters.
  const [memberId, setMemberId] = useState<number | "">("");
  const [actionFilter, setActionFilter] = useState("");
  const [fromDate, setFromDate] = useState(""); // yyyy-mm-dd
  const [toDate, setToDate] = useState("");

  const params = useMemo(() => {
    const sp = new URLSearchParams();
    if (memberId !== "") sp.set("memberId", String(memberId));
    if (actionFilter) sp.set("action", actionFilter);
    if (fromDate) sp.set("from", String(new Date(fromDate).getTime()));
    if (toDate) {
      // Treat the picker's date as inclusive end-of-day.
      sp.set("to", String(new Date(toDate).getTime() + 24 * 3600_000 - 1));
    }
    return sp.toString();
  }, [memberId, actionFilter, fromDate, toDate]);

  const { data: members } = useSWR<Member[]>(
    team && token ? ["members", team.id, token] : null,
    async () => {
      const res = await fetch(
        `${url}/api/dashboard/teams/${team!.id}/members`,
        { headers: { Authorization: `Bearer ${token}` } },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return (await res.json()) as Member[];
    },
  );

  const { data } = useSWR<AuditPage>(
    team && token ? ["audit", team.id, params, token] : null,
    async () => {
      const qs = params ? `?${params}` : "";
      const res = await fetch(
        `${url}/api/dashboard/teams/${team!.id}/get_audit_log_events${qs}`,
        { headers: { Authorization: `Bearer ${token}` } },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return (await res.json()) as AuditPage;
    },
  );

  const distinctActions = useMemo(() => {
    const set = new Set<string>();
    (data?.events ?? []).forEach((e) => set.add(e.action));
    return Array.from(set).sort();
  }, [data]);

  const rows = useMemo(() => groupNoise(data?.events ?? []), [data]);

  if (!mounted || !team || !token) return null;

  const hasFilters = memberId !== "" || actionFilter || fromDate || toDate;

  return (
    <TeamSettingsLayout page="audit-log" title="Audit Log">
      <Sheet>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h3 className="text-base font-semibold">Activity</h3>
            <p className="mt-1 max-w-prose text-sm text-content-secondary">
              Every state-changing action against the orchestrator gets recorded
              here, scoped to this team. Repeated actions by the same actor
              within 5 minutes are grouped.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <ExportButton
              rows={data?.events ?? []}
              members={members ?? []}
              format="csv"
              teamSlug={team.slug}
            />
            <ExportButton
              rows={data?.events ?? []}
              members={members ?? []}
              format="json"
              teamSlug={team.slug}
            />
          </div>
        </div>
        <div className="mt-4 flex flex-wrap items-end gap-3 rounded-md bg-background-tertiary/40 p-3">
          <label className="flex flex-col gap-1 text-xs">
            <span className="text-content-secondary">Actor</span>
            <select
              value={memberId === "" ? "" : String(memberId)}
              onChange={(e) =>
                setMemberId(e.target.value === "" ? "" : Number(e.target.value))
              }
              className="h-8 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            >
              <option value="">All members</option>
              {(members ?? []).map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name ?? m.email}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs">
            <span className="text-content-secondary">Action</span>
            <select
              value={actionFilter}
              onChange={(e) => setActionFilter(e.target.value)}
              className="h-8 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            >
              <option value="">All actions</option>
              {distinctActions.map((a) => (
                <option key={a} value={a}>
                  {humanizeAction(a)}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs">
            <span className="text-content-secondary">From</span>
            <input
              type="date"
              value={fromDate}
              onChange={(e) => setFromDate(e.target.value)}
              className="h-8 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs">
            <span className="text-content-secondary">To</span>
            <input
              type="date"
              value={toDate}
              onChange={(e) => setToDate(e.target.value)}
              className="h-8 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            />
          </label>
          {hasFilters && (
            // eslint-disable-next-line react/forbid-elements -- inline filter-clear pill, intentional plain <button>
            <button
              type="button"
              onClick={() => {
                setMemberId("");
                setActionFilter("");
                setFromDate("");
                setToDate("");
              }}
              className="text-xs text-content-secondary underline hover:text-content-primary"
            >
              Clear
            </button>
          )}
        </div>
        <ul className="mt-4 divide-y divide-border-transparent">
          {rows.map((row, i) =>
            row.kind === "single" ? (
              <SingleRow key={`s-${row.event.id}`} event={row.event} />
            ) : (
              <GroupRow key={`g-${row.first}-${i}`} group={row} />
            ),
          )}
          {rows.length === 0 && (
            <li className="py-3 text-sm text-content-secondary">
              {hasFilters
                ? "No activity matches the current filters."
                : "No activity yet."}
            </li>
          )}
        </ul>
      </Sheet>
    </TeamSettingsLayout>
  );
}

function SingleRow({ event }: { event: AuditEvent }) {
  return (
    <li className="flex flex-col gap-1 py-3 text-sm">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-baseline gap-2">
          <span className="font-medium text-content-primary">
            {humanizeAction(event.action)}
          </span>
          <span className="text-xs text-content-secondary">
            by {actorOf(event)}
          </span>
        </div>
        <span className="text-xs text-content-secondary">
          {new Date(event.creationTime).toLocaleString()}
        </span>
      </div>
      <MetaRow metadata={event.metadata} />
    </li>
  );
}

function GroupRow({ group }: { group: Extract<Row, { kind: "group" }> }) {
  return (
    <li className="flex flex-col gap-1 py-3 text-sm">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-baseline gap-2">
          <span className="font-medium text-content-primary">
            {humanizeAction(group.action)}
          </span>
          <span className="text-xs text-content-secondary">
            by {group.actor} · ×{group.count}
          </span>
        </div>
        <span className="text-xs text-content-secondary">
          {new Date(group.first).toLocaleString()} –{" "}
          {new Date(group.last).toLocaleTimeString()}
        </span>
      </div>
    </li>
  );
}

function MetaRow({ metadata }: { metadata: Record<string, unknown> }) {
  const interesting = Object.entries(metadata).filter(
    ([k]) =>
      ![
        "email",
        "method",
        "auth_user_id",
        "role", // role-only metadata is noisy on signin
      ].includes(k),
  );
  if (interesting.length === 0) return null;
  return (
    <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-content-secondary">
      {interesting.map(([k, v]) => (
        <span key={k}>
          <span className="text-content-tertiary">{k}:</span>{" "}
          <span className="font-mono text-content-primary">
            {typeof v === "string" ? v : JSON.stringify(v)}
          </span>
        </span>
      ))}
    </div>
  );
}

function ExportButton({
  rows,
  members,
  format,
  teamSlug,
}: {
  rows: AuditEvent[];
  members: Member[];
  format: "csv" | "json";
  teamSlug: string;
}) {
  const onClick = () => {
    const memberById = new Map(members.map((m) => [m.id, m]));
    const enriched = rows.map((e) => {
      const meta = e.metadata as { email?: string };
      const m = e.memberId !== null && e.memberId !== undefined
        ? memberById.get(e.memberId)
        : undefined;
      return {
        id: e.id,
        time: new Date(e.creationTime).toISOString(),
        action: e.action,
        actor_email: meta?.email ?? m?.email ?? "",
        actor_name: m?.name ?? "",
        member_id: e.memberId,
        metadata: e.metadata,
      };
    });

    let blob: Blob;
    let extension: string;
    if (format === "json") {
      blob = new Blob([JSON.stringify(enriched, null, 2)], {
        type: "application/json",
      });
      extension = "json";
    } else {
      const headers = [
        "id",
        "time",
        "action",
        "actor_email",
        "actor_name",
        "member_id",
        "metadata",
      ];
      const lines = [headers.join(",")];
      for (const row of enriched) {
        const cells = [
          row.id,
          row.time,
          row.action,
          row.actor_email,
          row.actor_name,
          row.member_id ?? "",
          JSON.stringify(row.metadata),
        ].map(csvCell);
        lines.push(cells.join(","));
      }
      blob = new Blob([lines.join("\n")], { type: "text/csv" });
      extension = "csv";
    }

    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    const filename = `audit-log-${teamSlug}-${stamp}.${extension}`;
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(link.href);
  };

  return (
    // eslint-disable-next-line react/forbid-elements -- compact 8px-tall download trigger; @ui/Button doesn't ship a "tiny secondary" variant
    <button
      type="button"
      onClick={onClick}
      disabled={rows.length === 0}
      className="inline-flex h-8 items-center rounded-md border border-border-transparent px-3 text-xs font-medium text-content-primary hover:bg-background-tertiary disabled:opacity-50"
    >
      Export {format.toUpperCase()}
    </button>
  );
}

// Wrap a CSV cell, escaping quotes and embedding commas/newlines safely.
function csvCell(value: string | number | null): string {
  const s = value === null || value === undefined ? "" : String(value);
  if (/[",\n]/.test(s)) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}
