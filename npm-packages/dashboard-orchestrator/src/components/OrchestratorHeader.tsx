// Top-level cloud-style header that renders on every dashboard-orchestrator
// page. Shows a combined team+project picker, deployment pill, and account
// menu mirroring https://dashboard.convex.dev's chrome.

import Link from "next/link";
import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import {
  CaretSortIcon,
  ExitIcon,
  GearIcon,
  MagnifyingGlassIcon,
  Pencil2Icon,
  PersonIcon,
  PlusIcon,
  ResetIcon,
} from "@radix-ui/react-icons";
import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import { Button } from "@ui/Button";
import { Menu, MenuItem, MenuLink } from "@ui/Menu";
import { Popover } from "@ui/Popover";
import { Tooltip } from "@ui/Tooltip";
import { ToggleTheme } from "@common/elements/ToggleTheme";
import { deploymentTypeColorClasses } from "@common/lib/deploymentTypeColorClasses";
import { Avatar } from "./Avatar";
import { ConvexOrb } from "./ConvexOrb";
import { CreateTeamModal } from "./CreateTeamModal";
import { NavBar } from "./NavBar";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { signOut, useSession } from "../lib/auth-client";
import { orchestratorUrl } from "../lib/config";
import {
  Deployment,
  Project,
  Team,
  listDeployments,
  listProjects,
  listTeams,
} from "../lib/orchestratorApi";

export function OrchestratorHeader() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
  const deploymentName = router.query.deploymentName as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();

  const { data: teams } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );
  const { data: projects } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );
  const project = useMemo(
    () => projects?.find((p) => p.slug === projectSlug),
    [projects, projectSlug],
  );
  const { data: deployments } = useSWR(
    project && token ? ["deployments", project.id, token] : null,
    () => listDeployments(url, token!, project!.id),
  );
  const deployment = useMemo(
    () => deployments?.find((d) => d.name === deploymentName),
    [deployments, deploymentName],
  );

  const onLogout = async () => {
    await signOut();
    void router.replace("/login");
  };

  if (process.env.NEXT_PUBLIC_HIDE_HEADER) return null;
  if (!mounted) {
    // Render an empty header shell during SSR/first paint to avoid
    // hydration mismatches between cookie-derived and URL-derived state.
    return (
      <header className="scrollbar-none flex min-h-[56px] items-center justify-between gap-2 border-b bg-background-secondary pr-2 pl-3" />
    );
  }

  // Mode the header is in:
  //  - team:       team home / team settings  → picker (team-only) + NavBar
  //  - project:    project home               → picker (team+project)
  //  - projectSettings: project settings      → picker + "Project settings" pill
  //  - deployment: deployment routes          → picker + deployment pill
  const isTeamLevel =
    !projectSlug &&
    (router.pathname === "/t/[team]" ||
      router.pathname.startsWith("/t/[team]/settings"));
  const isProjectSettings =
    !!project &&
    !deploymentName &&
    router.pathname.includes("[project]/settings");

  return (
    <header className="scrollbar-none flex min-h-[56px] items-center justify-between gap-2 overflow-x-auto border-b bg-background-secondary pr-2 pl-1 sm:gap-4">
      <div className="flex h-full items-center gap-2">
        <Link
          href="/"
          aria-label="Home"
          className="flex shrink-0 items-center justify-center px-2"
        >
          <ConvexOrb size={28} />
        </Link>
        {team && (
          <ProjectSelector
            team={team}
            project={project}
            teams={teams ?? []}
            projects={projects ?? []}
          />
        )}
        {team && isTeamLevel && (
          <NavBar
            items={[
              { label: "Home", href: `/t/${team.slug}` },
              { label: "Team Settings", href: `/t/${team.slug}/settings` },
            ]}
          />
        )}
        {team && project && isProjectSettings && (
          <ProjectSettingsPill
            href={`/t/${team.slug}/${project.slug}/settings`}
          />
        )}
        {team && project && deployment && (
          <DeploymentPill
            team={team}
            project={project}
            deployment={deployment}
            deployments={deployments ?? []}
          />
        )}
      </div>
      <div className="flex items-center gap-2">
        <UserMenu team={team} project={project} onLogout={onLogout} />
      </div>
    </header>
  );
}

// Combined team+project picker: header pill matches cloud's design
// (avatar + team-or-project name) and opens a popover that toggles between
// project list (default) and team list (when the team-pill is clicked).
function ProjectSelector({
  team,
  project,
  teams,
  projects,
}: {
  team: Team;
  project?: Project;
  teams: Team[];
  projects: Project[];
}) {
  const button = (
    <Button
      aria-label={project ? "Switch project" : "Switch to team selection"}
      variant="unstyled"
      type="button"
      className="flex h-10 w-fit cursor-pointer items-center gap-2 rounded-full px-3 py-2 text-content-primary select-none hover:bg-background-tertiary focus-visible:ring-2 focus-visible:ring-border-selected focus-visible:outline-none focus-visible:ring-inset"
    >
      <span className="flex items-center gap-2">
        <Avatar name={team.name} hashKey={team.id.toString()} />
        {project ? (
          <>
            <span className="text-content-secondary">/</span>
            <span className="max-w-56 truncate text-sm font-semibold">
              {project.name}
            </span>
          </>
        ) : (
          <span className="max-w-56 truncate text-sm font-semibold">
            {team.name}
          </span>
        )}
      </span>
      <CaretSortIcon className="size-5" />
    </Button>
  );
  return (
    <Popover
      placement="bottom-start"
      padding={false}
      portal
      openButtonClassName="bg-background-tertiary rounded-full"
      button={button}
    >
      {({ close }) => (
        <ProjectSelectorPanel
          team={team}
          teams={teams}
          projects={projects}
          close={close}
        />
      )}
    </Popover>
  );
}

function ProjectSelectorPanel({
  team,
  teams,
  projects,
  close,
}: {
  team: Team;
  teams: Team[];
  projects: Project[];
  close: () => void;
}) {
  const [switchingTeams, setSwitchingTeams] = useState(false);
  const [showCreateTeam, setShowCreateTeam] = useState(false);
  return (
    <>
      <div
        role="dialog"
        className="flex max-h-[calc(100vh-3.625rem)] w-86 flex-col py-2"
      >
        <div className="my-0.5 flex w-full items-center justify-between gap-2 px-0.5">
          <h5 className="mb-1 flex h-fit items-center gap-1 truncate">
            {switchingTeams ? (
              <div className="px-1.5 py-2 text-sm">Select Team</div>
            ) : (
              // eslint-disable-next-line react/forbid-elements -- compact team-switcher pill needs custom styling, not @ui/Button's chrome
              <button
                type="button"
                aria-label="Switch team"
                onClick={() => setSwitchingTeams(true)}
                className="mx-1.5 flex cursor-pointer items-center gap-1 rounded-full border px-1.5 py-1 hover:bg-background-tertiary"
              >
                <Avatar name={team.name} hashKey={team.id.toString()} />
                <span className="max-w-48 truncate">{team.name}</span>
                <CaretSortIcon className="min-h-4 min-w-4 rounded-full text-content-primary" />
              </button>
            )}
          </h5>
          {switchingTeams ? (
            <Button
              size="xs"
              onClick={() => setSwitchingTeams(false)}
              inline
              variant="neutral"
              icon={<ResetIcon />}
              tip="Back to projects"
              tipSide="right"
              aria-label="Back to projects"
            />
          ) : (
            <Button
              size="xs"
              href={`/t/${team.slug}/settings`}
              onClickOfAnchorLink={close}
              inline
              variant="neutral"
              icon={<GearIcon />}
              tip="Team settings"
              tipSide="right"
              aria-label={`Team settings for ${team.name}`}
            />
          )}
        </div>
        <div className="flex flex-col items-start gap-0.5 overflow-x-hidden">
          {switchingTeams ? (
            <TeamMenuOptions
              teams={teams}
              currentTeam={team}
              close={close}
              onCreateTeamClick={() => {
                close();
                setShowCreateTeam(true);
              }}
            />
          ) : (
            <ProjectMenuOptions team={team} projects={projects} close={close} />
          )}
        </div>
      </div>
      {showCreateTeam && (
        <CreateTeamModal onClose={() => setShowCreateTeam(false)} />
      )}
    </>
  );
}

function ProjectMenuOptions({
  team,
  projects,
  close,
}: {
  team: Team;
  projects: Project[];
  close: () => void;
}) {
  const router = useRouter();
  const currentSlug = router.query.project as string | undefined;
  const [query, setQuery] = useState("");
  const filtered = projects.filter(
    (p) =>
      !query ||
      p.name.toLowerCase().includes(query.toLowerCase()) ||
      p.slug.toLowerCase().includes(query.toLowerCase()),
  );
  const sorted = [
    ...filtered.filter((p) => p.slug === currentSlug),
    ...filtered.filter((p) => p.slug !== currentSlug),
  ];
  return (
    <>
      <div className="sticky top-0 z-10 flex w-full items-center gap-2 border-b bg-background-secondary px-3">
        <MagnifyingGlassIcon className="text-content-secondary" />
        <input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search projects..."
          className="w-full truncate bg-background-secondary py-1.5 text-left text-xs font-normal text-content-primary placeholder:text-content-tertiary focus:outline-hidden"
        />
      </div>
      <div className="px-2 pt-1 text-xs font-semibold text-content-secondary">
        Projects
      </div>
      <div
        className="scrollbar flex max-h-88 w-full flex-col overflow-y-auto p-0.5"
        role="menu"
      >
        {sorted.length === 0 ? (
          <div className="flex w-full items-center justify-center py-4 text-xs text-content-secondary">
            {query
              ? "No projects match your search."
              : `No projects in ${team.name} yet.`}
          </div>
        ) : (
          sorted.map((p) => (
            <Link
              key={p.id}
              href={`/t/${team.slug}/${p.slug}`}
              onClick={close}
              role="menuitem"
              className={`flex w-full items-center rounded-sm p-2 text-sm text-content-primary hover:bg-background-tertiary ${
                p.slug === currentSlug ? "bg-background-tertiary/60" : ""
              }`}
            >
              <span className="truncate">{p.name}</span>
            </Link>
          ))
        )}
      </div>
      <div className="flex w-full gap-2 p-2">
        <Button
          inline
          size="sm"
          icon={<PlusIcon />}
          className="grow"
          href={`/t/${team.slug}?create=1`}
          onClickOfAnchorLink={close}
        >
          Create Project
        </Button>
      </div>
    </>
  );
}

function TeamMenuOptions({
  teams,
  currentTeam,
  close,
  onCreateTeamClick,
}: {
  teams: Team[];
  currentTeam: Team;
  close: () => void;
  onCreateTeamClick: () => void;
}) {
  const sorted = [
    currentTeam,
    ...teams
      .filter((t) => t.id !== currentTeam.id)
      .sort((a, b) => a.name.localeCompare(b.name)),
  ];
  return (
    <>
      <div
        className="scrollbar flex w-full grow flex-col items-start gap-0.5 overflow-y-auto p-0.5"
        role="menu"
      >
        {sorted.map((t) => (
          <Link
            key={t.id}
            href={`/t/${t.slug}`}
            onClick={close}
            role="menuitem"
            className={`flex w-full items-center gap-2 rounded-sm p-2 text-sm text-content-primary hover:bg-background-tertiary ${
              t.id === currentTeam.id ? "bg-background-tertiary/60" : ""
            }`}
          >
            <Avatar name={t.name} hashKey={t.id.toString()} />
            <span className="grow truncate">{t.name}</span>
          </Link>
        ))}
      </div>
      <div className="px-2 pt-1 pb-2">
        <Button
          inline
          size="sm"
          icon={<PlusIcon />}
          className="w-full"
          onClick={onCreateTeamClick}
        >
          Create Team
        </Button>
      </div>
    </>
  );
}

function ProjectSettingsPill({ href }: { href: string }) {
  return (
    <Link
      href={href}
      className="flex h-9 items-center gap-2 rounded-full bg-background-tertiary px-3 py-1.5 text-sm font-medium text-content-primary hover:bg-background-primary"
    >
      <GearIcon className="size-4" />
      Project settings
    </Link>
  );
}

function DeploymentPill({
  team,
  project,
  deployment,
  deployments,
}: {
  team: Team;
  project: Project;
  deployment: Deployment;
  deployments: Deployment[];
}) {
  const kind = deployment.kind ?? deployment.deploymentType ?? "prod";
  return (
    <ColoredPill
      kind={kind}
      ariaLabel="Switch deployment"
      selected={
        <span className="flex items-center gap-2 text-sm">
          <KindIcon kind={kind} />
          <span>{deploymentLabel(kind)}</span>
          <span className="px-0.5 font-normal" role="separator">
            •
          </span>
          <span className="font-mono font-normal">{deployment.name}</span>
        </span>
      }
    >
      {(close) => (
        <DeploymentMenu
          team={team}
          project={project}
          deployment={deployment}
          deployments={deployments}
          close={close}
        />
      )}
    </ColoredPill>
  );
}

// Always-on deployment dropdown matching cloud: rows for Production,
// Development, Preview, Other, plus a Project Settings link. Rows that don't
// have a deployment yet still render and link to the project page so the
// user can provision one.
function DeploymentMenu({
  team,
  project,
  deployment,
  deployments,
  close,
}: {
  team: Team;
  project: Project;
  deployment: Deployment;
  deployments: Deployment[];
  close: () => void;
}) {
  const prod = deployments.find((d) => (d.kind ?? d.deploymentType) === "prod");
  const dev = deployments.find((d) => (d.kind ?? d.deploymentType) === "dev");
  const previews = deployments.filter(
    (d) => (d.kind ?? d.deploymentType) === "preview",
  );
  const others = deployments.filter(
    (d) =>
      (d.kind ?? d.deploymentType) !== "prod" &&
      (d.kind ?? d.deploymentType) !== "dev" &&
      (d.kind ?? d.deploymentType) !== "preview",
  );
  const provisionHref = (kind: "prod" | "dev" | "preview") =>
    `/t/${team.slug}/${project.slug}?provision=${kind}`;
  const dHref = (d: Deployment) => `/t/${team.slug}/${project.slug}/${d.name}`;
  return (
    <>
      <DeploymentMenuRow
        kind="prod"
        title="Production"
        selected={deployment.id === prod?.id}
        href={prod ? dHref(prod) : provisionHref("prod")}
        subtitle={prod ? prod.name : "Click to provision"}
        shortcut="Ctrl+Alt+1"
        close={close}
      />
      <DeploymentMenuRow
        kind="dev"
        title="Development"
        selected={deployment.id === dev?.id}
        href={dev ? dHref(dev) : provisionHref("dev")}
        subtitle={dev ? dev.name : "Click to provision"}
        shortcut="Ctrl+Alt+2"
        close={close}
      />
      <DeploymentMenuRow
        kind="preview"
        title="Preview Deployments"
        selected={previews.some((d) => d.id === deployment.id)}
        href={previews[0] ? dHref(previews[0]) : provisionHref("preview")}
        subtitle={
          previews.length
            ? `${previews.length} preview deployment${previews.length > 1 ? "s" : ""}`
            : "Click to provision"
        }
        close={close}
      />
      {others.length > 0 && (
        <DeploymentMenuRow
          kind="custom"
          title="Other Deployments"
          selected={others.some((d) => d.id === deployment.id)}
          href={dHref(others[0])}
          subtitle={`${others.length} deployment${others.length > 1 ? "s" : ""}`}
          close={close}
        />
      )}
      <div className="my-1 border-t" />
      <DeploymentMenuRow
        kind="settings"
        title="Project Settings"
        href={`/t/${team.slug}/${project.slug}/settings`}
        subtitle="Manage this project's configuration"
        shortcut="Ctrl+Alt+S"
        close={close}
      />
    </>
  );
}

function DeploymentMenuRow({
  kind,
  title,
  subtitle,
  href,
  selected,
  shortcut,
  external,
  close,
}: {
  kind: "prod" | "dev" | "preview" | "custom" | "settings";
  title: string;
  subtitle?: string;
  href: string;
  selected?: boolean;
  shortcut?: string;
  external?: boolean;
  close: () => void;
}) {
  return (
    <Link
      href={href}
      onClick={close}
      target={external ? "_blank" : undefined}
      rel={external ? "noopener noreferrer" : undefined}
      className={`mx-1 flex items-center gap-3 rounded-sm p-2 text-sm hover:bg-background-tertiary ${
        selected ? "bg-background-tertiary/60" : ""
      }`}
    >
      <RowIcon kind={kind} />
      <div className="flex min-w-0 flex-col">
        <span className="truncate text-content-primary">{title}</span>
        {subtitle && (
          <span className="truncate font-mono text-xs text-content-secondary">
            {subtitle}
          </span>
        )}
      </div>
      {shortcut && (
        <span className="ml-auto shrink-0 text-xs text-content-tertiary">
          {shortcut}
        </span>
      )}
    </Link>
  );
}

function RowIcon({
  kind,
}: {
  kind: "prod" | "dev" | "preview" | "custom" | "settings";
}) {
  const cls = "size-4 text-content-secondary shrink-0";
  if (kind === "prod") return <SignalIcon className={cls} />;
  if (kind === "dev") return <CommandLineIcon className={cls} />;
  if (kind === "preview") return <Pencil2Icon className={cls} />;
  if (kind === "custom") return <WrenchIcon className={cls} />;
  return <GearIcon className={cls} />;
}

function ColoredPill({
  kind,
  ariaLabel,
  selected,
  children,
}: {
  kind: string;
  ariaLabel: string;
  selected: React.ReactNode;
  children: (close: () => void) => React.ReactNode;
}) {
  const button = (
    <Button
      aria-label={ariaLabel}
      variant="unstyled"
      type="button"
      className={`flex h-9.25 w-fit cursor-pointer items-center gap-2 rounded-full border px-3 text-sm font-medium transition-opacity select-none hover:opacity-80 focus-visible:ring-1 focus-visible:ring-border-selected focus-visible:outline-hidden ${deploymentTypeColorClasses(
        (kind === "prod" || kind === "preview" || kind === "custom"
          ? kind
          : "dev") as "prod" | "preview" | "dev" | "custom",
      )}`}
    >
      {selected}
      <CaretSortIcon className="ml-auto size-5 shrink-0 bg-transparent" />
    </Button>
  );
  return (
    <Popover
      placement="bottom-start"
      padding={false}
      portal
      openButtonClassName="rounded-full"
      button={button}
    >
      {({ close }) => (
        <div className="flex w-[24rem] flex-col py-2">{children(close)}</div>
      )}
    </Popover>
  );
}

function KindIcon({ kind }: { kind: string }) {
  if (kind === "prod") return <SignalIcon className="size-4 min-w-4" />;
  if (kind === "preview") return <Pencil2Icon className="size-4 min-w-4" />;
  if (kind === "custom") return <WrenchIcon className="size-4 min-w-4" />;
  return <CommandLineIcon className="size-4 min-w-4" />;
}

function deploymentLabel(kind: string) {
  if (kind === "prod") return "Production";
  if (kind === "preview") return "Preview";
  return "Development";
}

// Profile dropdown matching cloud's UserMenu: name+email header,
// Profile Settings, Theme picker, Team Settings, Project Settings, Log Out.
function UserMenu({
  team,
  project,
  onLogout,
}: {
  team?: Team;
  project?: Project;
  onLogout: () => Promise<void>;
}) {
  const session = useSession();
  const user = session?.data?.user;
  const name = user?.name || user?.email || "?";
  const email = user?.email ?? "";
  return (
    <Menu
      buttonProps={{
        icon: (
          <span className="block">
            <Avatar name={name} hashKey={user?.id ?? email ?? "anon"} />
          </span>
        ),
        variant: "unstyled",
        className:
          "rounded-full p-1 transition-colors hover:bg-background-tertiary",
        "aria-label": "Account",
      }}
      placement="bottom-end"
    >
      <div className="flex max-w-[20rem] min-w-[20rem] flex-col gap-1 border-b px-3 pt-1 pb-2 text-content-primary">
        {user?.name && <div className="text-sm font-semibold">{user.name}</div>}
        <div
          className={
            user?.name
              ? "text-xs text-content-secondary"
              : "text-sm text-content-primary"
          }
        >
          {email || "Signed in"}
        </div>
      </div>
      <Tooltip
        side="left"
        tip="Settings related to your personal profile (e.g. name and email)."
      >
        <MenuLink href="/profile">
          <div className="flex w-full items-center justify-between">
            Profile Settings
            <PersonIcon className="text-content-secondary" />
          </div>
        </MenuLink>
      </Tooltip>
      <ToggleTheme />
      {team ? (
        <>
          <hr className="mx-4" />
          <Tooltip
            side="left"
            tip="Settings related to your team (members, billing, usage)."
          >
            <MenuLink href={`/t/${team.slug}/settings`}>
              <div className="flex w-full items-center justify-between gap-1">
                Team Settings
                <span className="max-w-24 truncate text-xs text-content-secondary">
                  {team.name}
                </span>
              </div>
            </MenuLink>
          </Tooltip>
          {project ? (
            <Tooltip
              side="left"
              tip="Settings related to your project (name, slug, access controls)."
            >
              <MenuLink href={`/t/${team.slug}/${project.slug}/settings`}>
                <div className="flex w-full items-center justify-between">
                  Project Settings
                  <span className="max-w-24 truncate text-xs text-content-secondary">
                    {project.name}
                  </span>
                </div>
              </MenuLink>
            </Tooltip>
          ) : null}
        </>
      ) : null}
      <hr className="mx-4" />
      <MenuItem action={onLogout}>
        <div className="flex w-full items-center justify-between">
          Log Out
          <ExitIcon className="text-content-secondary" />
        </div>
      </MenuItem>
    </Menu>
  );
}
