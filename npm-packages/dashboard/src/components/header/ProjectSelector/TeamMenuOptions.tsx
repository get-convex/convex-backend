import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { SelectorItem } from "elements/SelectorItem";
import { useCurrentTeam } from "api/teams";
import { useRouter } from "next/router";
import { Team } from "generatedApi";
import { Avatar } from "elements/Avatar";

export function TeamMenuOptions({
  teams,
  close,
  team,
  onCreateTeamClick,
}: {
  teams?: Team[];
  team: Team | null;
  onCreateTeamClick: () => void;
  close(): void;
}) {
  const { pathname } = useRouter();
  const currentTeam = useCurrentTeam();
  return (
    <>
      {teams && (
        <div
          className="scrollbar flex w-full grow flex-col items-start gap-0.5 overflow-y-auto p-0.5"
          role="menu"
        >
          {currentTeam && (
            <SelectorItem
              close={close}
              href={{
                pathname: pathname.startsWith("/t/[team]/settings")
                  ? pathname
                  : "/t/[team]",
                query: { team: currentTeam.slug },
              }}
              key={currentTeam?.slug}
              active={team?.slug === currentTeam.slug}
              selected
              eventName="switch team"
            >
              {/* Make room for the checkbox on selected items with this width calculation */}
              <div className="flex w-[calc(100%-0.75rem)] items-center gap-2">
                <Avatar
                  name={currentTeam.name}
                  hashKey={currentTeam.id.toString()}
                />
                <span className="grow truncate">{currentTeam.name}</span>
              </div>
            </SelectorItem>
          )}
          {teams
            .filter((t) => t.slug !== currentTeam?.slug)
            .sort((a, b) => a.name.localeCompare(b.name))
            .map((t) => (
              <SelectorItem
                close={close}
                href={{
                  pathname: pathname.startsWith("/t/[team]/settings")
                    ? pathname
                    : "/t/[team]",
                  query: { team: t.slug },
                }}
                key={t.slug}
                active={team?.slug === t.slug}
                eventName="switch team"
              >
                {/* Make room for the checkbox on selected items with this width calculation */}
                <div className="flex w-[calc(100%-0.75rem)] items-center gap-2">
                  <Avatar name={t.name} hashKey={t.id.toString()} />
                  <span className="grow truncate">{t.name}</span>
                </div>
              </SelectorItem>
            ))}
        </div>
      )}
      <Button
        inline
        onClick={() => {
          onCreateTeamClick();
          close();
        }}
        icon={<PlusIcon aria-hidden="true" />}
        className="w-full"
        size="sm"
      >
        Create Team
      </Button>
    </>
  );
}
