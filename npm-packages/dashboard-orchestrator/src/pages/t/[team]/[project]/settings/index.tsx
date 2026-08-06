import { useRouter } from "next/router";
import useSWR, { useSWRConfig } from "swr";
import { useEffect, useMemo, useState } from "react";
import Head from "next/head";
import { Button } from "@ui/Button";
import { Link as UiLink } from "@ui/Link";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TrashIcon } from "@radix-ui/react-icons";
import {
  listDeployments,
  listProjects,
  listTeams,
  Project,
  restartDeployment,
  Team,
} from "../../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../../lib/config";
import {
  BackendSettingsForm,
  type BackendSettingsDraft,
} from "../../../../../components/backendSettings/BackendSettingsForm";
import { tierDefaultsForName } from "../../../../../components/backendSettings/tiers";
import { useHostCapacity } from "../../../../../hooks/useHostCapacity";
import { useKnobRegistry } from "../../../../../hooks/useKnobRegistry";
import { useProjectSettings } from "../../../../../hooks/useProjectSettings";
import { CustomDomainsCard } from "../../../../../components/CustomDomainsCard";
import { DnsCredentialsCard } from "../../../../../components/DnsCredentialsCard";

const SECTION_IDS = {
  projectForm: "project-form",
  projectUsage: "project-usage",
  projectAdmins: "project-admins",
  backend: "backend",
  envVars: "env-vars",
  customDomains: "custom-domains",
  deleteProject: "delete-project",
} as const;

const sections: Array<{ id: string; label: string }> = [
  { id: SECTION_IDS.projectForm, label: "Edit Project" },
  { id: SECTION_IDS.projectUsage, label: "Project Usage" },
  { id: SECTION_IDS.projectAdmins, label: "Project Admins" },
  { id: SECTION_IDS.backend, label: "Backend" },
  { id: SECTION_IDS.envVars, label: "Environment Variables" },
  { id: SECTION_IDS.customDomains, label: "Custom Domains" },
  { id: SECTION_IDS.deleteProject, label: "Delete Project" },
];

export default function ProjectSettingsPage() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
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
  const { data: projects, mutate: mutateProjects } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );
  const project: Project | undefined = useMemo(
    () => projects?.find((p) => p.slug === projectSlug),
    [projects, projectSlug],
  );

  useEffect(() => {
    if (
      typeof window !== "undefined" &&
      window.location.hash &&
      team &&
      project
    ) {
      const id = window.location.hash.slice(1);
      const el = document.getElementById(id);
      if (el) {
        setTimeout(() => {
          el.scrollIntoView({ behavior: "smooth", block: "start" });
        }, 100);
      }
    }
  }, [team, project]);

  if (!mounted || !team || !project || !token) return null;

  return (
    <>
      <Head>
        <title>Project Settings | {project.name} | Convex Orchestrator</title>
      </Head>
      <div className="relative h-full [--container-px:--spacing(6)] [--container-width:80rem] [--sidebar-gap:--spacing(8)] [--sidebar-width:14rem]">
        <div className="pointer-events-none absolute inset-0 top-0 z-10 hidden md:block">
          <div className="mx-auto flex h-full max-w-(--container-width) gap-(--sidebar-gap) px-(--container-px)">
            <div className="h-full w-(--sidebar-width)">
              <div className="grid h-full grid-rows-[auto_1fr]">
                {/* eslint-disable-next-line no-restricted-syntax -- text-lg IS the heading style on this h2 */}
                <h2 className="pointer-events-auto py-6 text-lg font-semibold">
                  Project Settings
                </h2>
                <div className="scrollbar overflow-y-auto">
                  <div className="pointer-events-auto pb-8">
                    <SettingsNavigation />
                  </div>
                </div>
              </div>
            </div>
            <div className="grow" />
          </div>
        </div>
        <div
          className="scrollbar h-full overflow-y-auto"
          data-settings-content-wrapper
        >
          <div className="m-auto flex min-h-0 max-w-(--container-width) gap-(--sidebar-gap) px-(--container-px)">
            <div className="hidden w-(--sidebar-width) shrink-0 md:block" />
            <div className="flex grow flex-col items-start">
              <div className="md:hidden">
                {/* eslint-disable-next-line no-restricted-syntax -- text-lg IS the heading style on this h2 */}
                <h2 className="py-6 text-lg font-semibold">Project Settings</h2>
              </div>
              <div
                data-settings-content
                className="flex w-full grow flex-col gap-6 pr-2 pb-6 *:scroll-mt-3 md:pt-20"
              >
                <div id={SECTION_IDS.projectForm}>
                  <EditProjectSection
                    team={team}
                    project={project}
                    token={token}
                    url={url}
                    onUpdated={mutateProjects}
                  />
                </div>
                <div id={SECTION_IDS.projectUsage}>
                  <Sheet>
                    <h3 className="mb-4 text-base font-semibold">
                      Project Usage
                    </h3>
                    <p className="text-sm">
                      Self-hosted orchestrator deployments are unlimited. There
                      are no included or on-demand quotas to track.
                    </p>
                  </Sheet>
                </div>
                <div id={SECTION_IDS.projectAdmins}>
                  <ProjectAdminsSection
                    team={team}
                    project={project}
                    token={token}
                    url={url}
                  />
                </div>
                <div id={SECTION_IDS.backend}>
                  <BackendSection team={team} project={project} />
                </div>
                <div id={SECTION_IDS.envVars}>
                  <EnvVarsSection project={project} token={token} url={url} />
                </div>
                <div id={SECTION_IDS.customDomains}>
                  <CustomDomainsSection team={team} project={project} />
                </div>
                <div id={SECTION_IDS.deleteProject}>
                  <DeleteProjectSection
                    team={team}
                    project={project}
                    token={token}
                    url={url}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

function SettingsNavigation() {
  return (
    <nav
      data-settings-nav
      className="relative"
      aria-label="Settings navigation"
    >
      <div
        className="absolute left-0 h-full w-0.5 rounded-sm bg-background-tertiary"
        aria-hidden="true"
      />
      <SettingsNavigationScrollProgress />
      <ul className="pl-1 text-sm">
        {sections.map(({ id, label }) => (
          <li key={id} className="py-px">
            <a
              href={`#${id}`}
              className="block rounded-sm p-2 text-content-primary transition-all hover:bg-background-secondary"
              onClick={(e) => {
                e.preventDefault();
                const el = document.getElementById(id);
                if (el) {
                  const rect = el.getBoundingClientRect();
                  const isInView =
                    rect.top >= 0 && rect.bottom <= window.innerHeight;
                  el.scrollIntoView({
                    behavior: "smooth",
                    block: isInView ? "start" : "nearest",
                  });
                  window.history.pushState(null, "", `#${id}`);
                }
              }}
            >
              {label}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}

function SettingsNavigationScrollProgress() {
  const [transform, setTransform] = useState<string | undefined>(undefined);

  useEffect(() => {
    const wrapper = document.querySelector(
      "[data-settings-content-wrapper]",
    ) as HTMLElement | null;
    const content = document.querySelector(
      "[data-settings-content]",
    ) as HTMLElement | null;
    if (!wrapper) return undefined;

    const update = () => {
      const firstEl = document.getElementById(sections[0].id);
      if (!firstEl) {
        setTransform(undefined);
        return;
      }
      const containerRect = wrapper.getBoundingClientRect();
      const elementHeight = 1 / sections.length;
      const first = findScrollBoundary("first", containerRect);
      const last = findScrollBoundary("last", containerRect);
      const y = (first.index + first.topClippedFraction) * elementHeight;
      const h =
        first.index === last.index
          ? first.visibilityFraction * elementHeight
          : (first.visibilityFraction +
              last.visibilityFraction +
              last.index -
              first.index -
              1) *
            elementHeight;
      setTransform(`translateY(${y * 100}%) scaleY(${h})`);
    };

    update();

    const onScroll = () => window.requestAnimationFrame(update);
    wrapper.addEventListener("scroll", onScroll);
    window.addEventListener("resize", onScroll);
    const ro = new ResizeObserver(onScroll);
    if (content) ro.observe(content);

    return () => {
      wrapper.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      ro.disconnect();
    };
  }, []);

  if (transform === undefined) return null;
  return (
    <div
      className="absolute left-0 h-full w-0.5 origin-top rounded-sm bg-content-primary"
      style={{ transform }}
      aria-hidden="true"
    />
  );
}

function findScrollBoundary(
  boundary: "first" | "last",
  containerRect: DOMRect,
) {
  const start = boundary === "first" ? 0 : sections.length - 1;
  const inc = boundary === "first" ? 1 : -1;
  for (
    let i = start;
    boundary === "first" ? i < sections.length : i >= 0;
    i += inc
  ) {
    const el = document.getElementById(sections[i].id);
    if (!el) continue;
    const rect = el.getBoundingClientRect();
    const visibleHeight =
      Math.min(rect.bottom, containerRect.bottom) -
      Math.max(rect.top, containerRect.top);
    if (visibleHeight > 0) {
      const elementHeight = rect.height;
      return {
        index: i,
        visibilityFraction: visibleHeight / elementHeight,
        topClippedFraction:
          Math.max(0, containerRect.top - rect.top) / elementHeight,
      };
    }
  }
  return { index: 0, visibilityFraction: 0, topClippedFraction: 0 };
}

function EditProjectSection({
  team,
  project,
  token,
  url,
  onUpdated,
}: {
  team: Team;
  project: Project;
  token: string;
  url: string;
  onUpdated: () => void;
}) {
  const router = useRouter();
  const [name, setName] = useState(project.name);
  const [slug, setSlug] = useState(project.slug);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setName(project.name);
    setSlug(project.slug);
    // We intentionally only re-seed when the project identity changes;
    // resetting on every name/slug change would clobber unsaved edits.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.id]);

  const dirty = name !== project.name || slug !== project.slug;

  const onSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);
    try {
      const res = await fetch(`${url}/api/dashboard/projects/${project.id}`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ name, slug }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      onUpdated();
      if (slug !== project.slug) {
        await router.replace(`/t/${team.slug}/${slug}/settings`);
      }
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Sheet>
      <h3 className="mb-4 text-base font-semibold">Edit Project</h3>
      <form onSubmit={onSave} className="flex flex-col gap-4">
        <TextInput
          id="projectName"
          label="Project Name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <TextInput
          id="projectSlug"
          label="Project Slug"
          value={slug}
          onChange={(e) => setSlug(e.target.value)}
          description="Changing the project slug will require you to update any deploy keys currently in use."
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
  );
}

type EnvVar = { name: string; value: string; deploymentTypes: string[] };
type EnvVarsResponse = { variables: EnvVar[]; cursor: string | null };

function EnvVarsSection({
  project,
  token,
  url,
}: {
  project: Project;
  token: string;
  url: string;
}) {
  const { data, mutate } = useSWR<EnvVarsResponse>(
    ["env-vars", project.id, token],
    async () => {
      const res = await fetch(
        `${url}/v1/projects/${project.id}/list_default_environment_variables`,
        { headers: { Authorization: `Bearer ${token}` } },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return (await res.json()) as EnvVarsResponse;
    },
  );

  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const onAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const next = [
        ...(data?.variables ?? []).filter((v) => v.name !== name),
        { name, value, deploymentTypes: ["prod", "dev", "preview"] },
      ];
      const res = await fetch(
        `${url}/v1/projects/${project.id}/update_default_environment_variables`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ variables: next }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setName("");
      setValue("");
      await mutate();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  const onRemove = async (toRemove: string) => {
    setError(null);
    try {
      const next = (data?.variables ?? []).filter((v) => v.name !== toRemove);
      const res = await fetch(
        `${url}/v1/projects/${project.id}/update_default_environment_variables`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ variables: next }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutate();
    } catch (err) {
      setError((err as Error).message);
    }
  };

  return (
    <Sheet>
      <h3 className="mb-1 text-base font-semibold">
        Default Environment Variables
      </h3>
      <p className="max-w-prose text-sm text-content-secondary">
        These get applied to newly-provisioned deployments in this project.
        Existing deployments are not affected.
      </p>
      <form
        onSubmit={onAdd}
        className="mt-4 flex items-end gap-2 rounded-md bg-background-tertiary/40 p-3"
      >
        <div className="flex-1">
          <TextInput
            id="varName"
            label="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="MY_API_KEY"
          />
        </div>
        <div className="flex-1">
          <TextInput
            id="varValue"
            label="Value"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="sk-…"
          />
        </div>
        <Button
          type="submit"
          size="xs"
          disabled={!name || !value || submitting}
        >
          Add
        </Button>
      </form>
      {error && (
        <div className="mt-2 text-xs text-content-error" role="alert">
          {error}
        </div>
      )}
      <ul className="mt-4 divide-y divide-border-transparent">
        {(data?.variables ?? []).map((v) => (
          <li
            key={v.name}
            className="flex items-center justify-between gap-3 py-3"
          >
            <div className="min-w-0">
              <div className="truncate font-mono text-sm text-content-primary">
                {v.name}
              </div>
              <div className="truncate font-mono text-xs text-content-secondary">
                {v.value}
              </div>
            </div>
            <Button size="xs" variant="danger" onClick={() => onRemove(v.name)}>
              Remove
            </Button>
          </li>
        ))}
        {(data?.variables ?? []).length === 0 && (
          <li className="py-3 text-sm text-content-secondary">
            No default environment variables yet.
          </li>
        )}
      </ul>
    </Sheet>
  );
}

function DeleteProjectSection({
  team,
  project,
  token,
  url,
}: {
  team: Team;
  project: Project;
  token: string;
  url: string;
}) {
  const router = useRouter();
  const { mutate } = useSWRConfig();
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | undefined>();

  const onConfirm = async () => {
    setError(undefined);
    try {
      const res = await fetch(
        `${url}/api/dashboard/delete_project/${project.id}`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${token}` },
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      // Refresh the project + deployment lists so the deleted project
      // disappears from the team home without a manual reload. The team
      // page reads ["projects", teamId, token] and ["deployments", ...].
      await mutate(
        (key) =>
          Array.isArray(key) &&
          (key[0] === "projects" || key[0] === "deployments"),
        undefined,
        { revalidate: true },
      );
      void router.replace(`/t/${team.slug}`);
    } catch (err) {
      setError((err as Error).message);
      throw err;
    }
  };
  return (
    <Sheet>
      <h3 className="mb-2 text-base font-semibold">Delete Project</h3>
      <p className="mb-5 max-w-prose text-sm text-content-primary">
        Permanently delete this project for you and all team members. This
        action cannot be undone.
      </p>
      <Button
        variant="danger"
        size="xs"
        onClick={() => setOpen(true)}
        icon={<TrashIcon />}
      >
        Delete
      </Button>
      {open && (
        <ConfirmationDialog
          onClose={() => setOpen(false)}
          onConfirm={onConfirm}
          confirmText="Delete Project"
          dialogTitle="Delete Project"
          error={error}
          validationText={`Delete ${project.name} and data in all deployments, including Production`}
          dialogBody={
            <>
              Delete this project and all associated data.
              <div className="mt-2 font-semibold">
                Deleted projects cannot be recovered.
              </div>
            </>
          }
        />
      )}
    </Sheet>
  );
}

type ProjectMember = {
  id: number;
  email: string;
  name: string | null;
  role: string;
};

type ProjectRoleEntry = { projectId: number; memberId: number };

function ProjectAdminsSection({
  team,
  project,
  token,
  url,
}: {
  team: Team;
  project: Project;
  token: string;
  url: string;
}) {
  const { data: members } = useSWR<ProjectMember[]>(
    ["project-admins-members", team.id, token],
    () =>
      fetchJson<ProjectMember[]>(
        `${url}/api/dashboard/teams/${team.id}/members`,
        token,
      ),
  );
  const { data: roles, mutate } = useSWR<ProjectRoleEntry[]>(
    ["project-roles", team.id, token],
    () =>
      fetchJson<ProjectRoleEntry[]>(
        `${url}/api/dashboard/teams/${team.id}/get_project_roles`,
        token,
      ),
  );
  const [pendingError, setPendingError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Project admin = explicitly listed OR a team admin (team admins auto-grant
  // admin on every project). Cloud uses the same rule.
  const adminMemberIds = useMemo(() => {
    const fromRoles = (roles ?? [])
      .filter((r) => r.projectId === project.id)
      .map((r) => r.memberId);
    const teamAdminIds = (members ?? [])
      .filter((m) => m.role === "admin")
      .map((m) => m.id);
    return new Set<number>([...fromRoles, ...teamAdminIds]);
  }, [roles, members, project.id]);

  const explicitAdminIds = useMemo(
    () =>
      new Set(
        (roles ?? [])
          .filter((r) => r.projectId === project.id)
          .map((r) => r.memberId),
      ),
    [roles, project.id],
  );

  const toggleAdmin = async (memberId: number) => {
    setPendingError(null);
    const next = new Set(explicitAdminIds);
    if (next.has(memberId)) next.delete(memberId);
    else next.add(memberId);
    setSaving(true);
    try {
      const res = await fetch(
        `${url}/api/dashboard/teams/${team.id}/update_project_roles`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            projectId: project.id,
            adminMemberIds: Array.from(next),
          }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutate();
    } catch (err) {
      setPendingError((err as Error).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Sheet id={SECTION_IDS.projectAdmins}>
      <h3 className="mb-1 text-base font-semibold">Project Admins</h3>
      <p className="mb-4 max-w-prose text-sm text-content-secondary">
        Project admins can edit{" "}
        <span className="font-semibold">{project.name}</span> and its
        deployments. Team admins always have admin access on every project.
      </p>
      {pendingError && (
        <div className="mb-2 text-xs text-content-error" role="alert">
          {pendingError}
        </div>
      )}
      <ul className="divide-y divide-border-transparent">
        {(members ?? []).map((m) => {
          const isTeamAdmin = m.role === "admin";
          const isExplicitProjectAdmin = explicitAdminIds.has(m.id);
          const isAdmin = adminMemberIds.has(m.id);
          return (
            <li
              key={m.id}
              className="flex items-center justify-between gap-3 py-3"
            >
              <div className="min-w-0">
                <div className="truncate text-sm font-medium text-content-primary">
                  {m.name ?? m.email}
                </div>
                <div className="truncate text-xs text-content-secondary">
                  {m.email}
                  {isTeamAdmin && (
                    <span className="ml-2 rounded-sm bg-background-tertiary px-1.5 py-0.5 text-[10px] text-content-secondary uppercase">
                      Team admin
                    </span>
                  )}
                </div>
              </div>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={isAdmin}
                  disabled={isTeamAdmin || saving}
                  onChange={() => toggleAdmin(m.id)}
                />
                <span
                  className={
                    isExplicitProjectAdmin || isTeamAdmin
                      ? "text-content-primary"
                      : "text-content-secondary"
                  }
                >
                  Project admin
                </span>
              </label>
            </li>
          );
        })}
        {(members ?? []).length === 0 && (
          <li className="py-3 text-sm text-content-secondary">
            No team members yet.
          </li>
        )}
      </ul>
    </Sheet>
  );
}

// Custom domains attach to a *deployment* (that's what a hostname has to
// resolve to), but operators think about them per project — so the project
// settings page renders one card per deployment rather than making them go
// hunting through each deployment's settings.
function CustomDomainsSection({
  team,
  project,
}: {
  team: Team;
  project: Project;
}) {
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { data: deployments } = useSWR(
    token ? ["deployments", project.id, token] : null,
    () => listDeployments(url, token!, project.id),
  );

  if (deployments === undefined) {
    return (
      <Sheet>
        <h3>Custom Domains</h3>
        <p className="mt-2 text-sm text-content-secondary">Loading…</p>
      </Sheet>
    );
  }

  if (deployments.length === 0) {
    return (
      <Sheet>
        <h3>Custom Domains</h3>
        <p className="mt-2 text-sm text-content-secondary">
          This project has no deployments yet. Create one before attaching a
          custom domain.
        </p>
      </Sheet>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {deployments.map((d) => (
        <CustomDomainsCard
          key={d.id}
          deploymentId={d.id}
          deploymentName={d.name}
          teamId={team.id}
          heading={
            deployments.length > 1
              ? `Custom Domains — ${d.deploymentType ?? d.kind ?? d.name}`
              : "Custom Domains"
          }
        />
      ))}
      {/* Credentials are team-scoped and shared by every domain, so they sit
          below the per-deployment cards rather than inside each one. */}
      <DnsCredentialsCard teamId={team.id} />
    </div>
  );
}

function BackendSection({ team, project }: { team: Team; project: Project }) {
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { settings, save } = useProjectSettings(project.id);
  const { data: capacity } = useHostCapacity();
  const { data: registry } = useKnobRegistry();
  // The production deployment's running tier — used to subtract its
  // slice from host-capacity allocation so the strip doesn't double-count
  // when an operator changes the project tier to resize prod. Falls back
  // to undefined (no subtraction) if the project has no prod deployment
  // yet or while the list is loading.
  const { data: deployments, mutate: mutateDeployments } = useSWR(
    token ? ["deployments", project.id, token] : null,
    () => listDeployments(url, token!, project.id),
  );
  const productionDeployment = useMemo(
    () =>
      (deployments ?? []).find(
        (d) => d.deploymentType === "prod" || d.kind === "prod",
      ),
    [deployments],
  );
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [restartOpen, setRestartOpen] = useState(false);
  const [restartMessage, setRestartMessage] = useState<string | null>(null);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [draft, setDraft] = useState<BackendSettingsDraft | null>(null);

  useEffect(() => {
    if (settings && draft === null) {
      setDraft({
        tier: settings.tier,
        overrides: { ...settings.knobOverrides },
      });
    }
  }, [settings, draft]);

  const dirty =
    !!settings &&
    !!draft &&
    (draft.tier !== settings.tier ||
      JSON.stringify(draft.overrides) !==
        JSON.stringify(settings.knobOverrides));
  const draftTier = draft?.tier;
  const tierDefaults = useMemo(
    () => (draftTier ? tierDefaultsForName(draftTier) : {}),
    [draftTier],
  );

  const onSave = async () => {
    if (!draft) return;
    setError(null);
    setRestartMessage(null);
    setSaving(true);
    try {
      const patch: Record<string, string | null> = { ...draft.overrides };
      if (settings) {
        for (const k of Object.keys(settings.knobOverrides)) {
          if (!(k in draft.overrides)) patch[k] = null;
        }
      }
      await save({ tier: draft.tier, knobOverrides: patch });
      setSavedAt(Date.now());
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const onRestartDeployments = async () => {
    if (!token || !deployments?.length) return;
    setError(null);
    setRestartMessage(null);
    setRestarting(true);
    try {
      for (const deployment of deployments) {
        await restartDeployment(url, token, deployment.name);
      }
      await mutateDeployments();
      setRestartMessage(
        `Restart requested for ${deployments.length} deployment${
          deployments.length === 1 ? "" : "s"
        }.`,
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRestarting(false);
    }
  };

  if (!draft) {
    return (
      <Sheet>
        <h3 className="mb-4 text-base font-semibold">Backend</h3>
        <p className="text-sm text-content-secondary">Loading…</p>
      </Sheet>
    );
  }

  return (
    <Sheet>
      <h3 className="mb-4 text-base font-semibold">Backend</h3>
      <BackendSettingsForm
        registry={registry}
        capacity={capacity}
        tierDefaults={tierDefaults}
        currentTier={productionDeployment?.tier}
        initial={draft}
        onChange={setDraft}
      />
      {savedAt && !dirty && (
        <div className="mt-3 text-xs text-content-secondary">
          <p>
            Saved. New deployments will use these settings. Existing deployments
            keep running with their current container settings until restarted.
          </p>
          {!!deployments?.length && (
            <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
              <p>
                Restart existing deployment containers to apply the new
                settings. Each deployment will be offline for the duration of
                its restart.
              </p>
              <Button
                size="xs"
                variant="neutral"
                onClick={() => setRestartOpen(true)}
                disabled={restarting}
              >
                {restarting ? "Restarting…" : "Restart deployments"}
              </Button>
            </div>
          )}
          {restartMessage && (
            <p className="mt-2 text-content-success" role="status">
              {restartMessage}
            </p>
          )}
        </div>
      )}
      {error && (
        <div className="mt-2 text-xs text-content-error" role="alert">
          {error}
        </div>
      )}
      <div className="mt-4 flex items-center justify-between">
        <UiLink
          href={`/t/${team.slug}/${project.slug}/settings/advanced-knobs`}
        >
          View advanced settings →
        </UiLink>
        <Button size="xs" onClick={onSave} disabled={!dirty || saving}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </div>
      {restartOpen && (
        <ConfirmationDialog
          dialogTitle="Restart deployments"
          dialogBody={
            <>
              <p className="text-sm">
                Restarting will recreate the existing deployment containers with
                the saved backend settings.
              </p>
              <p className="mt-3 text-sm font-semibold">
                Each deployment will be offline for the duration of its restart.
              </p>
            </>
          }
          confirmText="Restart deployments"
          onClose={() => setRestartOpen(false)}
          onConfirm={onRestartDeployments}
          variant="primary"
        />
      )}
    </Sheet>
  );
}

async function fetchJson<T>(u: string, token: string): Promise<T> {
  const res = await fetch(u, { headers: { Authorization: `Bearer ${token}` } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return (await res.json()) as T;
}
