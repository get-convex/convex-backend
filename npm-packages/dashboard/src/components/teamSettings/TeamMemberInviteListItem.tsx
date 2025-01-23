import { Button } from "dashboard-common";
import { Cross2Icon, ReloadIcon } from "@radix-ui/react-icons";
import startCase from "lodash/startCase";
import {
  CancelInvitationArgs,
  CreateInvitationArgs,
  InvitationResponse,
} from "generatedApi";

type TeamMemberInviteListItemProps = {
  invite: InvitationResponse;
  hasAdminPermissions: boolean;
  onCreateInvite: (body: CreateInvitationArgs) => void;
  onCancelInvite: (body: CancelInvitationArgs) => void;
};

export function TeamMemberInviteListItem({
  invite,
  hasAdminPermissions,
  onCreateInvite,
  onCancelInvite,
}: TeamMemberInviteListItemProps) {
  return (
    <div className="flex items-center gap-4 border-b py-2 last:border-b-0">
      <div className="flex flex-col">
        <div className="text-sm text-content-secondary">{invite.email}</div>
      </div>
      <div className="ml-auto mr-2 flex items-center gap-2">
        {invite.expired && (
          <span className="text-sm font-semibold text-content-error">
            Invitation expired
          </span>
        )}
        <span className="text-sm text-content-secondary">
          {startCase(invite.role)}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <Button
          onClick={() =>
            onCreateInvite({ email: invite.email, role: invite.role })
          }
          icon={<ReloadIcon />}
          variant="neutral"
          disabled={!hasAdminPermissions}
          tip={
            !hasAdminPermissions
              ? "You do not have permission to invite team members"
              : undefined
          }
        >
          Resend
        </Button>
        <Button
          onClick={() => onCancelInvite({ email: invite.email })}
          variant="danger"
          icon={<Cross2Icon />}
          disabled={!hasAdminPermissions}
          tip={
            !hasAdminPermissions
              ? "You do not have permission to cancel invitations"
              : undefined
          }
        >
          Revoke
        </Button>
      </div>
    </div>
  );
}
