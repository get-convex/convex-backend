import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { TeamResponse } from "generatedApi";
import VercelLogo from "logos/vercel.svg";

export function OpenInVercel({ team }: { team: TeamResponse }) {
  if (team.managedBy !== "vercel" || !team.managedByUrl) {
    return null;
  }

  return (
    <Button
      href={team.managedByUrl}
      target="_blank"
      size="sm"
      variant="neutral"
      icon={<VercelLogo className="size-3 fill-[#EDEDED]" />}
      className="bg-[#1A1A1A] text-[#EDEDED] hover:bg-[#1A1A1A] hover:opacity-40"
      tip="This team is managed by Vercel. Visit the Vercel dashboard to manage your team and create new projects."
    >
      Open in Vercel
      <ExternalLinkIcon className="ml-1 size-3" />
    </Button>
  );
}
