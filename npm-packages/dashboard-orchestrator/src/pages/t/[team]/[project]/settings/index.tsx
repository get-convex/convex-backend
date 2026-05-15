import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import Head from "next/head";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { Modal } from "@ui/Modal";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Link as UiLink } from "@ui/Link";
import { CopyButton } from "@common/elements/CopyButton";
import { TrashIcon } from "@radix-ui/react-icons";
import {
  listDeployments,
  listProjects,
  listTeams,
  Project,
  Team,
  Deployment,
} from "../../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../../lib/config";

const SECTION_IDS = {
  projectForm: "project-form",
  projectUsage: "project-usage",
  projectAdmins: "project-admins",
  productionDeployKeys: "production-deploy-keys",
  previewDeployKeys: "preview-deploy-keys",
  envVars: "env-vars",
  deleteProject: "delete-project",
} as const;

const sections: Array<{ id: string; label: string }> = [
  { id: SECTION_IDS.projectForm, label: "Edit Project" },
  { id: SECTION_IDS.projectUsage, label: "Project Usage" },
  { id: SECTION_IDS.projectAdmins, label: "Project Admins" },
  { id: SECTION_IDS.productionDeployKeys, label: "Production Deploy Keys" },
  { id: SECTION_IDS.previewDeployKeys, label: "Preview Deploy Keys" },
  { id: SECTION_IDS.envVars, label: "Environment Variables" },
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
                className="flex w-full grow flex-col gap-6 pr-2 pb-6 md:pt-20 [&>*]:scroll-mt-3"
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
                <div id={SECTION_IDS.productionDeployKeys}>
                  <DeployKeysSection
                    team={team}
                    project={project}
                    token={token}
                    url={url}
                    kind="prod"
                  />
                </div>
                <div id={SECTION_IDS.previewDeployKeys}>
                  <DeployKeysSection
                    team={team}
                    project={project}
                    token={token}
                    url={url}
                    kind="preview"
                  />
                </div>
                <div id={SECTION_IDS.envVars}>
                  <EnvVarsSection project={project} token={token} url={url} />
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
              className="block rounded-sm px-2 py-2 text-content-primary transition-all hover:bg-background-secondary"
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

type DeployKey = {
  id: string;
  name: string;
  creationTime: number;
  keySuffix: string;
  /** Milliseconds since epoch; absent if the key never expires. */
  expiresAt?: number;
};

function DeployKeysSection({
  team: _team,
  project,
  token,
  url,
  kind,
}: {
  team: Team;
  project: Project;
  token: string;
  url: string;
  kind: "prod" | "preview";
}) {
  const { data: deployments } = useSWR(["deployments", project.id, token], () =>
    listDeployments(url, token, project.id),
  );
  const target: Deployment | undefined = deployments?.find(
    (d) => (d.kind ?? d.deploymentType) === kind,
  );

  const { data: keys, mutate } = useSWR<DeployKey[]>(
    target && token ? ["deployKeys", kind, target.name, token] : null,
    async () => {
      const all = await fetchJson<DeployKey[]>(
        `${url}/v1/deployments/${target!.name}/list_deploy_keys`,
        token,
      );
      return all.filter((k) => k.name !== "ephemeral");
    },
  );

  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  // "" = never expire, otherwise yyyy-mm-dd from <input type="date">.
  const [newExpiry, setNewExpiry] = useState("");
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [revoking, setRevoking] = useState<DeployKey | null>(null);
  const [revokeError, setRevokeError] = useState<string | undefined>();

  const onCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!target) return;
    setError(null);
    setSubmitting(true);
    try {
      const expiresAt = newExpiry
        ? // Treat the picker's date as end-of-day local so a key dated
          // 2026-05-31 is valid through that whole day.
          new Date(newExpiry).getTime() + 24 * 3600_000 - 1
        : undefined;
      const res = await fetch(
        `${url}/v1/deployments/${target.name}/create_deploy_key`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ name: newName, expires_at: expiresAt }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const body = (await res.json()) as { key: string };
      setCreatedKey(body.key);
      setNewName("");
      setNewExpiry("");
      setShowCreate(false);
      await mutate();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  const onConfirmRevoke = async () => {
    if (!target || !revoking) return;
    setRevokeError(undefined);
    try {
      const res = await fetch(
        `${url}/v1/deployments/${target.name}/delete_deploy_key`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ id: revoking.id }),
        },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      await mutate();
    } catch (err) {
      setRevokeError((err as Error).message);
      throw err;
    }
  };

  const heading =
    kind === "prod" ? "Production Deploy Keys" : "Preview Deploy Keys";
  const description =
    kind === "prod" ? (
      <>
        Used by{" "}
        <code className="rounded-sm bg-background-tertiary px-1 font-mono">
          npx convex deploy
        </code>{" "}
        to push to production.
      </>
    ) : (
      <>
        Used by hosting providers (Vercel, Netlify) to{" "}
        <UiLink
          href="https://docs.convex.dev/production/multiple-deployments#preview"
          target="_blank"
        >
          create preview deployments
        </UiLink>{" "}
        for pull requests.
      </>
    );

  return (
    <>
      <Sheet id={`section-${kind}-deploy-keys`}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="mb-1 text-base font-semibold">{heading}</h3>
            <p className="max-w-prose text-sm text-content-secondary">
              {description}
            </p>
          </div>
          <Button
            size="xs"
            onClick={() => setShowCreate(true)}
            disabled={!target}
          >
            + Generate
          </Button>
        </div>
        {!target && (
          <p className="mt-3 text-sm text-content-secondary">
            Provision a {kind === "prod" ? "production" : "preview"} deployment
            first.
          </p>
        )}
        {error && (
          <div className="mt-2 text-xs text-content-error" role="alert">
            {error}
          </div>
        )}
        <ul className="mt-4 divide-y divide-border-transparent">
          {(keys ?? []).map((k) => {
            const expired =
              k.expiresAt !== null &&
              k.expiresAt !== undefined &&
              k.expiresAt < Date.now();
            return (
              <li
                key={k.id}
                className="flex items-center justify-between gap-3 py-3"
              >
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium text-content-primary">
                    {k.name}
                    {expired && (
                      <span className="rounded-full border border-current px-1.5 py-0.5 text-[10px] font-medium text-content-error uppercase">
                        Expired
                      </span>
                    )}
                  </div>
                  <div className="font-mono text-xs text-content-secondary">
                    {kind}:…{k.keySuffix}
                  </div>
                  {k.expiresAt !== null && k.expiresAt !== undefined && (
                    <div className="text-xs text-content-tertiary">
                      {expired ? "Expired " : "Expires "}
                      {new Date(k.expiresAt).toLocaleDateString(undefined, {
                        year: "numeric",
                        month: "short",
                        day: "numeric",
                      })}
                    </div>
                  )}
                </div>
                <Button
                  size="xs"
                  variant="danger"
                  onClick={() => setRevoking(k)}
                >
                  Revoke
                </Button>
              </li>
            );
          })}
          {(keys ?? []).length === 0 && target && (
            <li className="py-3 text-sm text-content-secondary">
              No deploy keys yet.
            </li>
          )}
        </ul>
      </Sheet>

      {showCreate && (
        <Modal
          title={`Generate ${kind} deploy key`}
          onClose={() => setShowCreate(false)}
        >
          <form onSubmit={onCreate} className="flex flex-col gap-4">
            <TextInput
              id={`${kind}KeyName`}
              label="Name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. CI"
              autoFocus
            />
            <label
              className="flex flex-col gap-1 text-sm"
              htmlFor={`${kind}KeyExpiry`}
            >
              <span className="text-content-primary">Expires (optional)</span>
              <input
                id={`${kind}KeyExpiry`}
                type="date"
                value={newExpiry}
                onChange={(e) => setNewExpiry(e.target.value)}
                min={new Date().toISOString().slice(0, 10)}
                className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
              />
              <span className="text-xs text-content-secondary">
                Leave blank for a key that never expires.
              </span>
            </label>
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
                {submitting ? "Generating…" : "Generate"}
              </Button>
            </div>
          </form>
        </Modal>
      )}

      {revoking && (
        <ConfirmationDialog
          dialogTitle="Revoke deploy key"
          confirmText="Revoke key"
          onClose={() => setRevoking(null)}
          onConfirm={onConfirmRevoke}
          error={revokeError}
          dialogBody={
            <>
              Revoke the {kind === "prod" ? "production" : "preview"} deploy key{" "}
              <span className="font-semibold">{revoking.name}</span>. Anything
              using this key (CI, hosting providers) will start failing.
            </>
          }
        />
      )}

      {createdKey && (
        <Modal title="Deploy key generated" onClose={() => setCreatedKey(null)}>
          <p className="mb-3 text-sm text-content-secondary">
            Copy this key now — you won't see it again.
          </p>
          <div className="flex items-center gap-2">
            <code className="flex-1 truncate rounded-sm bg-background-tertiary p-2 font-mono text-xs">
              {createdKey}
            </code>
            <CopyButton text={createdKey} />
          </div>
          <div className="mt-4 flex justify-end">
            <Button size="xs" onClick={() => setCreatedKey(null)}>
              Done
            </Button>
          </div>
        </Modal>
      )}
    </>
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

async function fetchJson<T>(u: string, token: string): Promise<T> {
  const res = await fetch(u, { headers: { Authorization: `Bearer ${token}` } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return (await res.json()) as T;
}
