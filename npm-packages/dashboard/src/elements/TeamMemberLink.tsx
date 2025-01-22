import { useCurrentTeam } from "api/teams";
import Link from "next/link";

export function TeamMemberLink({
  memberId,
  name,
}: {
  memberId?: number | null;
  name: string;
}) {
  const team = useCurrentTeam();
  return !memberId ? (
    <div className="font-semibold">Convex</div>
  ) : (
    <Link
      target="_blank"
      className="font-semibold text-content-link hover:underline dark:underline"
      href={`/t/${team?.slug}/settings/members#${memberId}`}
    >
      {name}
    </Link>
  );
}
