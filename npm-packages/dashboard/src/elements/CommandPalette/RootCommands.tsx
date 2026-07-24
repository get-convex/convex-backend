import { Command } from "cmdk";
import { useRouter } from "next/router";
import {
  CaretSortIcon,
  ExitIcon,
  Half2Icon,
  PersonIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { SparklesIcon } from "@heroicons/react/24/outline";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useSupportFormOpen } from "elements/SupportWidget";
import { openAskAI } from "elements/AskAI";
import { logout } from "lib/logout";
import {
  deploymentNavigation,
  deploymentSectionNavigation,
  findCurrentPage,
  NavigationDestination,
  NavigationTarget,
  profileSectionNavigation,
  projectNavigation,
  projectSectionNavigation,
  teamNavigation,
  teamSectionNavigation,
} from "./navigation";
import { ActionItem, NavigationItem } from "./items";
import {
  SwitchComponentItem,
  SwitchComponentSearchItems,
} from "./ComponentCommands";
import { DeploymentSearchGroup, ProjectSearchGroup } from "./searchGroups";
import { PalettePage } from "./pages";

const PROFILE_TARGET: NavigationTarget = {
  label: "Profile Settings",
  href: "/profile",
  Icon: PersonIcon,
};

export function RootCommands({
  search,
  onNavigate,
  pushPage,
  onClose,
}: {
  search: string;
  onNavigate: (to: NavigationDestination) => void;
  pushPage: (page: PalettePage) => void;
  onClose: () => void;
}) {
  const router = useRouter();
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const { usageLimits } = useLaunchDarkly();
  const [, setSupportFormOpen] = useSupportFormOpen();

  const deploymentName =
    typeof router.query.deploymentName === "string"
      ? router.query.deploymentName
      : undefined;

  const deploymentUriPrefix =
    team && project && deploymentName
      ? `/t/${team.slug}/${project.slug}/${deploymentName}`
      : undefined;
  const deploymentNav =
    deploymentUriPrefix && project
      ? deploymentNavigation(deploymentUriPrefix, {
          usageLimitsEnabled: usageLimits,
        })
      : undefined;
  // Sections within pages are only surfaced while searching, to keep the
  // browsable (empty-search) list scannable.
  const showSections = search.trim().length > 0;
  const projectNav =
    team && project
      ? projectNavigation(team.slug, project.slug, project.name)
      : undefined;
  const teamNav = team ? teamNavigation(team.slug, team.name) : undefined;

  const currentPage = findCurrentPage(
    [
      ...(deploymentNav
        ? [...deploymentNav.pages, ...deploymentNav.settings]
        : []),
      ...(projectNav ?? []),
      ...(teamNav ?? []),
      PROFILE_TARGET,
    ],
    router.asPath,
  );

  return (
    <>
      {currentPage && (
        <Command.Group heading="Current Page">
          <NavigationItem
            target={currentPage}
            onNavigate={onNavigate}
            hint="Current Page"
          />
        </Command.Group>
      )}
      {deploymentNav && project && (
        <Command.Group heading="Deployment">
          {[
            ...deploymentNav.pages,
            ...deploymentNav.settings,
            ...(showSections && deploymentUriPrefix
              ? deploymentSectionNavigation(deploymentUriPrefix)
              : []),
          ]
            // The current page is already pinned at the top.
            .filter((target) => target !== currentPage)
            .map((target) => (
              <NavigationItem
                key={target.label}
                target={target}
                onNavigate={onNavigate}
              />
            ))}
          <SwitchComponentItem
            onSelect={() => pushPage({ type: "components" })}
          />
          {showSections && <SwitchComponentSearchItems onClose={onClose} />}
        </Command.Group>
      )}
      {team && project && projectNav && (
        <Command.Group heading="Project">
          {[
            ...projectNav,
            ...(showSections
              ? projectSectionNavigation(team.slug, project.slug)
              : []),
          ]
            .filter((target) => target !== currentPage)
            .map((target) => (
              <NavigationItem
                key={target.label}
                target={target}
                onNavigate={onNavigate}
              />
            ))}
          <ActionItem
            value="page:switch-deployment"
            onSelect={() => pushPage({ type: "deployments", project })}
            Icon={CaretSortIcon}
            label="Switch Deployment…"
            drillIn
          />
          <SwitchProjectItem pushPage={pushPage} />
        </Command.Group>
      )}
      {team && teamNav && (
        <Command.Group heading="Team">
          {[
            ...teamNav,
            ...(showSections ? teamSectionNavigation(team.slug) : []),
          ]
            .filter((target) => target !== currentPage)
            .map((target) => (
              <NavigationItem
                key={target.label}
                target={target}
                onNavigate={onNavigate}
              />
            ))}
          {!project && (
            <SwitchProjectItem pushPage={pushPage} label="Go to Project…" />
          )}
          <ActionItem
            value="page:teams"
            onSelect={() => pushPage({ type: "teams" })}
            Icon={CaretSortIcon}
            label="Switch Team…"
            drillIn
          />
        </Command.Group>
      )}
      {team && (
        <ProjectSearchGroup
          team={team}
          search={search}
          onNavigate={onNavigate}
          pushPage={pushPage}
        />
      )}
      {team && search.trim() && (
        <DeploymentSearchGroup
          team={team}
          // Inside a project, only search that project's deployments.
          project={project}
          search={search.trim()}
          onNavigate={onNavigate}
          pushPage={pushPage}
        />
      )}
      <Command.Group heading="Account">
        {currentPage !== PROFILE_TARGET && (
          <NavigationItem target={PROFILE_TARGET} onNavigate={onNavigate} />
        )}
        {showSections &&
          profileSectionNavigation().map((target) => (
            <NavigationItem
              key={target.label}
              target={target}
              onNavigate={onNavigate}
            />
          ))}
        <ActionItem
          value="page:theme"
          onSelect={() => pushPage({ type: "theme" })}
          Icon={Half2Icon}
          label="Change Dashboard Theme…"
          drillIn
        />
        <ActionItem
          value="action:log-out"
          onSelect={() => void logout()}
          Icon={ExitIcon}
          label="Log Out"
        />
      </Command.Group>
      <Command.Group heading="Help">
        <ActionItem
          value="action:ask-ai"
          onSelect={() => {
            onClose();
            openAskAI();
          }}
          Icon={SparklesIcon}
          label="Ask AI"
        />
        <ActionItem
          value="action:contact-support"
          onSelect={() => {
            onClose();
            setSupportFormOpen(true);
          }}
          Icon={QuestionMarkCircledIcon}
          label="Contact Support"
        />
      </Command.Group>
    </>
  );
}

function SwitchProjectItem({
  pushPage,
  // "Switch" only makes sense when a project is currently selected.
  label = "Switch Project…",
}: {
  pushPage: (page: PalettePage) => void;
  label?: string;
}) {
  return (
    <ActionItem
      value="page:projects"
      onSelect={() => pushPage({ type: "projects" })}
      Icon={CaretSortIcon}
      label={label}
      drillIn
    />
  );
}
