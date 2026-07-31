import { Command } from "cmdk";
import { GearIcon, PlusIcon } from "@radix-ui/react-icons";
import { useTeams } from "api/teams";
import { Avatar } from "elements/Avatar";
import { useCreateTeamModalOpen } from "hooks/useCreateTeamModal";
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
              <Command.Item
                key={team.id}
                value={`team:${team.slug}`}
                className="animate-fadeInFromLoading"
                keywords={[team.name, team.slug]}
                onSelect={() => {
                  trackSelected("switch-team");
                  onNavigate(`/t/${team.slug}`);
                }}
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
                {team.slug === selectedTeamSlug && (
                  <span className="ml-auto text-xs text-content-tertiary">
                    <CurrentBadge />
                  </span>
                )}
              </Command.Item>
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
