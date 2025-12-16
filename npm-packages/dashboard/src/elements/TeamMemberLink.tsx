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
    <span className="font-semibold">Convex</span>
  ) : (
    <Link
      target="_blank"
      className="font-semibold text-content-link hover:underline"
      href={`/t/${team?.slug}/settings/members#${memberId}`}
    >
      {name}
    </Link>
  );
}
