import { useRouter } from "next/router";
import Link from "next/link";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Menu, MenuItem } from "@ui/Menu";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { SegmentedControl } from "@ui/SegmentedControl";
import { cn } from "@ui/cn";
import { EmptySection } from "@common/elements/EmptySection";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  DotsVerticalIcon,
  ExternalLinkIcon,
  GearIcon,
  GridIcon,
  ListBulletIcon,
  MagnifyingGlassIcon,
  PlusIcon,
  TrashIcon,
} from "@radix-ui/react-icons";
import {
  listDeployments,
  listDeploymentsForTeam,
  listProjects,
  listTeams,
  Project,
  Team,
  Deployment,
} from "../../../lib/orchestratorApi";
import { useAccessToken } from "../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../lib/config";
import { CreateProjectModal } from "../../../components/CreateProjectModal";

type View = "grid" | "list";
type Tab = "projects" | "deployments";

export default function TeamHomePage() {
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
  const { data: projects, mutate } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );

  const [tab, setTab] = useState<Tab>("projects");
  const [view, setView] = useState<View>("grid");
  const [search, setSearch] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [pageSize, setPageSize] = useState(24);
  const [page, setPage] = useState(1);

  if (!mounted || !team || !token) return null;

  const filtered = (projects ?? []).filter(
    (p) =>
      !search ||
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      p.slug.toLowerCase().includes(search.toLowerCase()),
  );
  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const safePage = Math.min(page, pageCount);
  const paged = filtered.slice((safePage - 1) * pageSize, safePage * pageSize);

  const isDeployments = tab === "deployments";

  return (
    <main className="flex flex-1 flex-col overflow-y-auto">
      <div className="mx-auto flex w-full max-w-7xl flex-col gap-4 p-6">
        <div className="flex w-full animate-fadeInFromLoading flex-col gap-3">
          <div className="flex items-center gap-4">
            <SegmentedControl
              options={[
                { label: "Projects", value: "projects" },
                { label: "Deployments", value: "deployments" },
              ]}
              value={tab}
              onChange={(v) => setTab(v as Tab)}
            />
            {!isDeployments && (
              <div className="ml-auto flex items-center gap-2">
                <Button
                  size="sm"
                  variant="neutral"
                  onClick={() => setShowCreate(true)}
                  icon={<PlusIcon />}
                >
                  Create Project
                </Button>
                <Button
                  size="sm"
                  href="https://docs.convex.dev/tutorial/"
                  target="_blank"
                  icon={<ExternalLinkIcon />}
                >
                  Start Tutorial
                </Button>
              </div>
            )}
          </div>
          {!isDeployments && (
            <div className="mt-1 flex items-center gap-2">
              <div className="w-52 shrink-0">
                <TextInput
                  id="searchProjects"
                  type="search"
                  label="Search projects"
                  labelHidden
                  placeholder="Search projects"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  SearchIcon={MagnifyingGlassIcon}
                />
              </div>
              <div className="hidden gap-1 rounded-md border bg-background-secondary p-1 lg:flex">
                <Button
                  icon={<GridIcon />}
                  variant="neutral"
                  inline
                  size="xs"
                  className={cn(view === "grid" && "bg-background-tertiary")}
                  onClick={() => setView("grid")}
                />
                <Button
                  icon={<ListBulletIcon />}
                  variant="neutral"
                  inline
                  size="xs"
                  className={cn(view === "list" && "bg-background-tertiary")}
                  onClick={() => setView("list")}
                />
              </div>
            </div>
          )}
        </div>

        {isDeployments ? (
          <DeploymentsTab
            team={team}
            projects={projects ?? []}
            token={token}
            url={url}
          />
        ) : (projects?.length ?? 0) === 0 && !search ? (
          <ProjectsEmptyState />
        ) : view === "grid" ? (
          <ProjectGrid
            team={team}
            projects={paged}
            token={token}
            url={url}
            onChanged={mutate}
          />
        ) : (
          <ProjectList
            team={team}
            projects={paged}
            token={token}
            url={url}
            onChanged={mutate}
          />
        )}

        {filtered.length === 0 && !isDeployments && search && (
          <div className="text-sm text-content-secondary">
            No projects match &quot;{search}&quot;.
          </div>
        )}

        {!isDeployments && filtered.length > 0 && (
          <Paginator
            page={safePage}
            pageCount={pageCount}
            pageSize={pageSize}
            onPageChange={setPage}
            onPageSizeChange={(n) => {
              setPageSize(n);
              setPage(1);
            }}
          />
        )}
      </div>

      {showCreate && (
        <CreateProjectModal
          team={team}
          onClose={() => setShowCreate(false)}
          onCreated={() => mutate()}
        />
      )}
    </main>
  );
}

function ProjectGrid({
  team,
  projects,
  token,
  url,
  onChanged,
}: {
  team: Team;
  projects: Project[];
  token: string;
  url: string;
  onChanged: () => void;
}) {
  return (
    <ul className="grid grid-cols-1 gap-4 lg:grid-cols-2 xl:grid-cols-3">
      {projects.map((p) => (
        <li key={p.id}>
          <ProjectCard
            team={team}
            project={p}
            token={token}
            url={url}
            onChanged={onChanged}
          />
        </li>
      ))}
    </ul>
  );
}

function ProjectList({
  team,
  projects,
  token,
  url,
  onChanged,
}: {
  team: Team;
  projects: Project[];
  token: string;
  url: string;
  onChanged: () => void;
}) {
  return (
    <div className="w-full overflow-hidden rounded-xl bg-background-secondary ring-1 ring-border-transparent">
      {projects.map((p, i) => (
        <div
          key={p.id}
          className={`first:rounded-t-xl last:rounded-b-xl ${
            i > 0 ? "border-t" : ""
          }`}
        >
          <ProjectCard
            team={team}
            project={p}
            token={token}
            url={url}
            onChanged={onChanged}
            listItem
          />
        </div>
      ))}
    </div>
  );
}

function ProjectCard({
  team,
  project,
  token,
  url,
  onChanged,
  listItem,
}: {
  team: Team;
  project: Project;
  token: string;
  url: string;
  onChanged: () => void;
  listItem?: boolean;
}) {
  return (
    <div
      className={`group relative ${
        listItem
          ? "bg-background-secondary"
          : "rounded-md border border-border-transparent bg-background-secondary hover:border-content-primary"
      }`}
    >
      <Link
        href={`/t/${team.slug}/${project.slug}`}
        className="flex items-center justify-between gap-3 p-4 pr-12"
      >
        <div className="min-w-0">
          <div className="truncate text-base font-semibold text-content-primary">
            {project.name}
          </div>
          <div className="truncate text-xs text-content-secondary">
            {project.slug}
          </div>
        </div>
        <div className="flex flex-col items-end gap-0.5">
          <DeploymentSummary token={token} url={url} projectId={project.id} />
          <span className="text-xs text-content-secondary">
            Created {timeAgo(project.creationTime)}
          </span>
        </div>
      </Link>
      <div className="absolute top-2 right-2">
        <ProjectCardMenu
          team={team}
          project={project}
          token={token}
          url={url}
          onChanged={onChanged}
        />
      </div>
    </div>
  );
}

function ProjectCardMenu({
  team,
  project,
  token,
  url,
  onChanged,
}: {
  team: Team;
  project: Project;
  token: string;
  url: string;
  onChanged: () => void;
}) {
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [error, setError] = useState<string | undefined>();

  const onConfirmDelete = async () => {
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
      onChanged();
    } catch (err) {
      setError((err as Error).message);
      throw err;
    }
  };
  return (
    <>
      <Menu
        buttonProps={{
          icon: (
            <DotsVerticalIcon className="size-7 rounded-sm p-1 text-content-secondary hover:bg-background-tertiary" />
          ),
          variant: "unstyled",
          "aria-label": `Project menu for ${project.name}`,
        }}
        placement="bottom-end"
      >
        <MenuItem href={`/t/${team.slug}/${project.slug}/settings`}>
          <span className="flex w-full items-center gap-2">
            <GearIcon className="size-4 text-content-secondary" />
            Project Settings
          </span>
        </MenuItem>
        <MenuItem variant="danger" action={() => setDeleteOpen(true)}>
          <span className="flex w-full items-center gap-2">
            <TrashIcon className="size-4" />
            Delete Project
          </span>
        </MenuItem>
      </Menu>
      {deleteOpen && (
        <DeleteProjectDialog
          project={project}
          onClose={() => setDeleteOpen(false)}
          onConfirm={onConfirmDelete}
          error={error}
        />
      )}
    </>
  );
}

function DeleteProjectDialog({
  project,
  onClose,
  onConfirm,
  error,
}: {
  project: Project;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  error?: string;
}) {
  // Cloud uses the literal phrase "Delete <Project Name> and data in all
  // deployments, including Production" so muscle-memory carries over.
  const validationText = `Delete ${project.name} and data in all deployments, including Production`;
  return (
    <ConfirmationDialog
      onClose={onClose}
      onConfirm={onConfirm}
      validationText={validationText}
      confirmText="Delete Project"
      dialogTitle="Delete Project"
      error={error}
      dialogBody={
        <>
          Delete this project and all associated data.
          <div className="mt-2 font-semibold">
            Deleted projects cannot be recovered.
          </div>
        </>
      }
    />
  );
}

function DeploymentSummary({
  token,
  url,
  projectId,
}: {
  token: string;
  url: string;
  projectId: number;
}) {
  const { data } = useSWR(["dep-summary", projectId, token], () =>
    listDeployments(url, token, projectId),
  );
  if (!data) return null;
  const kinds = new Set(data.map((d) => d.kind ?? d.deploymentType ?? "prod"));
  const labels: string[] = [];
  if (kinds.has("prod")) labels.push("Production");
  if (kinds.has("dev")) labels.push("Development");
  if (kinds.has("preview")) labels.push("Preview");
  return (
    <div className="flex flex-wrap items-center gap-1 text-xs text-content-secondary">
      {labels.map((l, i) => (
        <span key={l} className="flex items-center gap-1">
          {i > 0 && <span className="text-content-tertiary">·</span>}
          <span className="font-medium text-content-primary">{l}</span>
        </span>
      ))}
      {labels.length === 0 && <span>No deployments</span>}
    </div>
  );
}

function DeploymentsTab({
  team,
  projects,
  token,
  url,
}: {
  team: Team;
  projects: Project[];
  token: string;
  url: string;
}) {
  // Flat list of every deployment across the team, mirroring the cloud
  // dashboard's Deployments view. Cloud uses `useDeploymentsWithFilters`
  // (project / kind / search). We do the same filtering client-side off the
  // single team-level fetch.
  const { data: deployments } = useSWR(
    token ? ["team-deployments", team.id, token] : null,
    () => listDeploymentsForTeam(url, token, team.id),
  );
  const [projectFilter, setProjectFilter] = useState<number | "all">("all");
  const [kindFilter, setKindFilter] = useState<
    "all" | "prod" | "dev" | "preview" | "custom"
  >("all");
  const [search, setSearch] = useState("");

  const projectsById = useMemo(
    () => new Map(projects.map((p) => [p.id, p])),
    [projects],
  );
  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = (deployments ?? []).map((d) => ({
      deployment: d,
      project: projectsById.get(d.projectId),
    }));
    const filtered = list.filter(({ deployment, project }) => {
      if (!project) return false;
      if (projectFilter !== "all" && project.id !== projectFilter) return false;
      const kind = (deployment.kind ?? deployment.deploymentType ?? "prod") as
        | "prod"
        | "dev"
        | "preview"
        | "custom";
      if (kindFilter !== "all" && kind !== kindFilter) return false;
      if (
        q &&
        !deployment.name.toLowerCase().includes(q) &&
        !project.name.toLowerCase().includes(q)
      ) {
        return false;
      }
      return true;
    });
    filtered.sort(
      (a, b) =>
        (b.deployment.creationTime ?? 0) - (a.deployment.creationTime ?? 0),
    );
    return filtered;
  }, [deployments, projectsById, projectFilter, kindFilter, search]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-2">
        <div className="w-[18rem]">
          <TextInput
            id="searchDeployments"
            type="search"
            label="Search deployments"
            labelHidden
            placeholder="Search deployments"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            SearchIcon={MagnifyingGlassIcon}
          />
        </div>
        <FilterSelect
          label="Project"
          value={projectFilter === "all" ? "" : String(projectFilter)}
          onChange={(v) => setProjectFilter(v === "" ? "all" : Number(v))}
          options={[
            { value: "", label: "All projects" },
            ...projects.map((p) => ({ value: String(p.id), label: p.name })),
          ]}
        />
        <FilterSelect
          label="Kind"
          value={kindFilter === "all" ? "" : kindFilter}
          onChange={(v) =>
            setKindFilter(
              v === "" ? "all" : (v as "prod" | "dev" | "preview" | "custom"),
            )
          }
          options={[
            { value: "", label: "All kinds" },
            { value: "prod", label: "Production" },
            { value: "dev", label: "Development" },
            { value: "preview", label: "Preview" },
            { value: "custom", label: "Custom" },
          ]}
        />
      </div>
      {rows.length === 0 ? (
        <div className="py-8 text-center text-sm text-content-secondary">
          {(deployments?.length ?? 0) === 0
            ? "No deployments yet."
            : "No deployments match the current filters."}
        </div>
      ) : (
        <div className="w-full overflow-hidden rounded-xl bg-background-secondary ring-1 ring-border-transparent">
          {rows.map((r, i) =>
            r.project ? (
              <DeploymentRow
                key={r.deployment.id}
                team={team}
                project={r.project}
                deployment={r.deployment}
                divider={i > 0}
              />
            ) : null,
          )}
        </div>
      )}
    </div>
  );
}

function FilterSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <label className="flex flex-col gap-1 text-xs">
      <span className="text-content-secondary">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="h-8 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function DeploymentRow({
  team,
  project,
  deployment,
  divider,
}: {
  team: Team;
  project: Project;
  deployment: Deployment;
  divider: boolean;
}) {
  const kind = deployment.kind ?? deployment.deploymentType ?? "prod";
  return (
    <Link
      href={`/t/${team.slug}/${project.slug}/${deployment.name}`}
      className={`flex items-center gap-4 px-4 py-3 hover:bg-background-tertiary/40 ${
        divider ? "border-t" : ""
      }`}
    >
      <DeploymentKindBadge kind={kind} />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-content-primary">
            {project.name}
          </span>
          <span className="text-xs text-content-tertiary">/</span>
          <span className="truncate font-mono text-sm text-content-secondary">
            {deployment.name}
          </span>
        </div>
        {deployment.url && (
          <span className="truncate text-xs text-content-tertiary">
            {deployment.url}
          </span>
        )}
      </div>
      <span className="shrink-0 text-xs text-content-tertiary">
        {timeAgo(deployment.creationTime ?? Date.now())}
      </span>
    </Link>
  );
}

function DeploymentKindBadge({ kind }: { kind: string }) {
  const label =
    kind === "prod"
      ? "Production"
      : kind === "preview"
        ? "Preview"
        : kind === "custom"
          ? "Custom"
          : "Development";
  const color =
    kind === "prod"
      ? "bg-purple-600/20 text-purple-300"
      : kind === "preview"
        ? "bg-orange-600/20 text-orange-300"
        : kind === "custom"
          ? "bg-blue-600/20 text-blue-300"
          : "bg-green-600/20 text-green-300";
  return (
    <span
      className={`shrink-0 rounded-full px-2 py-0.5 text-xs font-medium ${color}`}
    >
      {label}
    </span>
  );
}

function Paginator({
  page,
  pageCount,
  pageSize,
  onPageChange,
  onPageSizeChange,
}: {
  page: number;
  pageCount: number;
  pageSize: number;
  onPageChange: (n: number) => void;
  onPageSizeChange: (n: number) => void;
}) {
  return (
    <div className="mt-2 flex items-center justify-end gap-4 text-xs text-content-secondary">
      <label className="flex items-center gap-2">
        Page size:
        <select
          value={pageSize}
          onChange={(e) => onPageSizeChange(Number(e.target.value))}
          className="rounded-sm border border-border-transparent bg-background-primary px-1 py-0.5 text-xs"
        >
          {[12, 24, 48, 96].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </label>
      <div className="flex items-center gap-1">
        <span>
          Page {page} of {pageCount}
        </span>
        {/* eslint-disable-next-line react/forbid-elements -- 24px icon-only pagination chevron, finer than @ui/Button's smallest variant */}
        <button
          type="button"
          aria-label="Previous page"
          disabled={page <= 1}
          onClick={() => onPageChange(page - 1)}
          className="inline-flex size-6 items-center justify-center rounded-sm hover:bg-background-tertiary disabled:opacity-40"
        >
          <ChevronLeftIcon className="size-4" />
        </button>
        {/* eslint-disable-next-line react/forbid-elements -- 24px icon-only pagination chevron, finer than @ui/Button's smallest variant */}
        <button
          type="button"
          aria-label="Next page"
          disabled={page >= pageCount}
          onClick={() => onPageChange(page + 1)}
          className="inline-flex size-6 items-center justify-center rounded-sm hover:bg-background-tertiary disabled:opacity-40"
        >
          <ChevronRightIcon className="size-4" />
        </button>
      </div>
    </div>
  );
}

function ProjectsEmptyState() {
  // Cloud uses EmptySection from dashboard-common for this state. We pass
  // sheet={false} so the centered card sits inside our existing layout
  // padding instead of stacking another Sheet wrapper.
  return (
    <EmptySection
      header="Welcome to Convex!"
      sheet={false}
      body={
        <>
          <p className="text-sm">
            This team doesn&apos;t have any projects yet.
          </p>
          <p className="text-sm">Get started by following the tutorial.</p>
        </>
      }
      action={
        <Button
          href="https://docs.convex.dev/tutorial/"
          target="_blank"
          icon={<ExternalLinkIcon />}
          className="mt-2"
        >
          Start Tutorial
        </Button>
      }
    />
  );
}

function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  if (days < 1) return "today";
  if (days === 1) return "yesterday";
  if (days < 30) return `${days} days ago`;
  if (days < 365) return `${Math.floor(days / 30)} months ago`;
  return `${Math.floor(days / 365)} years ago`;
}
