import { Command } from "cmdk";
import { useTeams } from "api/teams";
import { Avatar } from "elements/Avatar";
import { HighlightedText, LoadingSignal } from "./items";

// The drilled-into "Switch Team" page.
export function TeamsCommands({
  onNavigate,
}: {
  onNavigate: (href: string) => void;
}) {
  const { teams, selectedTeamSlug } = useTeams();

  if (!teams) {
    return <LoadingSignal />;
  }

  return (
    <Command.Group heading="Teams">
      {teams.map((team) => (
        <Command.Item
          key={team.id}
          value={`team:${team.slug}`}
          className="animate-fadeInFromLoading"
          keywords={[team.name, team.slug]}
          onSelect={() => onNavigate(`/t/${team.slug}`)}
        >
          <Avatar name={team.name} hashKey={team.id.toString()} />
          <span className="flex min-w-0 items-baseline gap-1.5">
            <span className="truncate">
              <HighlightedText text={team.name} />
            </span>
            <span className="truncate text-xs text-content-tertiary">
              <HighlightedText text={team.slug} />
            </span>
          </span>
          {team.slug === selectedTeamSlug && (
            <span className="ml-auto rounded-sm border px-1.5 py-0.5 text-xs text-content-tertiary">
              Current
            </span>
          )}
        </Command.Item>
      ))}
    </Command.Group>
  );
}
