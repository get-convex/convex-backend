import type { TeamResponse } from "generatedApi";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { DirectorySyncSheet } from "./DirectorySyncSheet";
import { DomainsSheet } from "./DomainsSheet";
import { SingleSignOnSheet } from "./SingleSignOnSheet";

export function TeamSSO({ team }: { team: TeamResponse }) {
  const { directorySync } = useLaunchDarkly();

  return (
    <div className="flex max-w-2xl flex-col gap-8">
      <h2>Team Authentication</h2>

      <DomainsSheet team={team} />
      <SingleSignOnSheet team={team} />
      {directorySync && <DirectorySyncSheet team={team} />}
    </div>
  );
}
