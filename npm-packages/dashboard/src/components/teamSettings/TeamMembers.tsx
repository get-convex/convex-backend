import { EnvelopeClosedIcon } from "@radix-ui/react-icons";
import { Callout } from "dashboard-common/elements/Callout";
import { Loading } from "dashboard-common/elements/Loading";
import { useTeamMembers, useTeamEntitlements } from "api/teams";
import { useTeamInvites } from "api/invitations";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import Link from "next/link";
import { Team } from "generatedApi";

import { InviteMemberForm } from "./InviteMemberForm";
import { TeamMemberList } from "./TeamMemberList";

export function TeamMembers({ team }: { team: Team }) {
  const members = useTeamMembers(team.id);
  const invites = useTeamInvites(team.id);
  const entitlements = useTeamEntitlements(team.id);

  const isLoading = !(members && invites && entitlements !== undefined);
  const canAddMembers =
    !isLoading && members.length < entitlements.maxTeamMembers!;

  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();

  let inviteMembers = null;
  if (isLoading) {
    // Data isn't loaded yet, show a skeleton.
    inviteMembers = (
      <Loading className="h-[9.5rem] w-full rounded" fullHeight={false} />
    );
  } else if (canAddMembers) {
    // Show invite form if you can add members.
    inviteMembers = (
      <InviteMemberForm
        team={team}
        members={members}
        hasAdminPermissions={hasAdminPermissions}
      />
    );
  } else if (members.length >= entitlements.maxTeamMembers!) {
    // Show an action item to upgrade if you have reached your member limit.
    inviteMembers = (
      <Callout>
        <div className="flex flex-col gap-2 p-2">
          <div>
            You've reached the member limit for this team.{" "}
            <Link
              href={`/${team.slug}/settings/billing`}
              className="items-center text-content-link dark:underline"
            >
              Upgrade
            </Link>{" "}
            to invite more team members.
          </div>
        </div>
      </Callout>
    );
  } else {
    // We've enumerated every case except for inactive subscriptions (canceled, delinquent, etc.),
    // so show an error message.
    inviteMembers = (
      <Callout variant="error">
        <div className="flex flex-col gap-2 p-2">
          <div>Your subscription is inactive.</div>
          <div>
            Contact us at{" "}
            <Link
              href="mailto:support@convex.dev"
              passHref
              className="items-center text-content-link dark:underline"
            >
              <EnvelopeClosedIcon className="mr-0.5 inline" />
              support@convex.dev
            </Link>{" "}
            to resolve this issue.
          </div>
        </div>
      </Callout>
    );
  }

  return (
    <>
      <h2>Members</h2>
      {inviteMembers}
      <TeamMemberList team={team} members={members} invites={invites} />
    </>
  );
}
