import Link from "next/link";
import { useCurrentTeam } from "../api/teams";

export function TeamMemberLink({
  memberId,
  name,
}: {
  memberId?: number | null;
  name: string;
}) {
  const team = useCurrentTeam();
  return !memberId ? (
    <span>Convex</span>
  ) : (
    <Link
      target="_blank"
      className="rounded text-content-link hover:underline focus-visible:outline-2 focus-visible:outline-border-selected"
      href={`/t/${team?.slug}/settings/members#${memberId}`}
    >
      {name}
    </Link>
  );
}
