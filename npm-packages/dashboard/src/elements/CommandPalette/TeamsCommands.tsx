import { Command } from "cmdk";
import { GearIcon, PlusIcon } from "@radix-ui/react-icons";
import type { TeamResponse } from "generatedApi";
import { useTeams } from "api/teams";
import { Avatar } from "elements/Avatar";
import { useCreateTeamModalOpen } from "hooks/useCreateTeamModal";
import { useRouter } from "next/router";
import { useCopyAction } from "./copy";
import { teamSwitchDestination } from "./navigation";
import {
  ActionItem,
  CurrentBadge,
  HighlightedText,
  LoadingSignal,
  NavigationItem,
  PinnedActions,
} from "./items";
import { usePaletteAnalytics } from "./analytics";

// The drilled-into "Switch Team" page.
export function TeamsCommands({
  onNavigate,
  onClose,
  contextual,
}: {
  onNavigate: (href: string) => void;
  onClose: () => void;
  // Only the anchored team-switcher menu shows the Team Settings shortcut; the
  // main palette's Switch Team page omits it.
  contextual: boolean;
}) {
  const { teams, selectedTeamSlug } = useTeams();
  const { trackSelected } = usePaletteAnalytics();
  const [, setCreateTeamOpen] = useCreateTeamModalOpen();
  const { pathname } = useRouter();

  const currentTeam = teams?.find((t) => t.slug === selectedTeamSlug);

  return (
    <>
      {!teams ? (
        <LoadingSignal />
      ) : (
        <>
          {contextual && currentTeam && (
            <Command.Group heading="Team">
              <NavigationItem
                target={{
                  label: "Team Settings",
                  href: `/t/${currentTeam.slug}/settings`,
                  Icon: GearIcon,
                }}
                onNavigate={onNavigate}
              />
            </Command.Group>
          )}
          <Command.Group heading="Switch Team">
            {teams.map((team) => (
              <TeamItem
                key={team.id}
                team={team}
                isCurrent={team.slug === selectedTeamSlug}
                onSelect={() => {
                  trackSelected("switch-team");
                  onNavigate(teamSwitchDestination(team.slug, pathname));
                }}
              />
            ))}
          </Command.Group>
        </>
      )}
      <PinnedActions>
        <ActionItem
          value="action:create-team"
          onSelect={() => {
            trackSelected("create-team");
            onClose();
            setCreateTeamOpen(true);
          }}
          Icon={PlusIcon}
          label="Create Team…"
        />
      </PinnedActions>
    </>
  );
}

function TeamItem({
  team,
  isCurrent,
  onSelect,
}: {
  team: TeamResponse;
  isCurrent: boolean;
  onSelect: () => void;
}) {
  const value = `team:${team.slug}`;
  useCopyAction(value, { label: "slug", getText: () => team.slug });
  return (
    <Command.Item
      value={value}
      className="animate-fadeInFromLoading"
      keywords={[team.name, team.slug]}
      onSelect={onSelect}
    >
      <Avatar name={team.name} hashKey={team.id.toString()} />
      <span className="flex min-w-0 flex-col">
        <span className="truncate">
          <HighlightedText text={team.name} />
        </span>
        <span className="truncate text-xs text-content-tertiary">
          <HighlightedText text={team.slug} />
        </span>
      </span>
      {isCurrent && (
        <span className="ml-auto text-xs text-content-tertiary">
          <CurrentBadge />
        </span>
      )}
    </Command.Item>
  );
}
