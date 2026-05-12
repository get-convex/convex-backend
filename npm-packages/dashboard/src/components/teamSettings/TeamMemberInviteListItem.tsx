import { Menu, MenuItem } from "@ui/Menu";
import { DotsVerticalIcon } from "@radix-ui/react-icons";
import {
  CancelInvitationArgs,
  CreateInvitationArgs,
  InvitationResponse,
} from "generatedApi";
import { RoleDisplay } from "./RoleDisplay";

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
  const onResend = () => {
    if (invite.role === "custom") {
      onCreateInvite({
        email: invite.email,
        role: "custom",
        customRoles: (invite.customRoles ?? []).map((r) => r.id),
      });
    } else {
      onCreateInvite({ email: invite.email, role: invite.role });
    }
  };
  const noPermissionTip = !hasAdminPermissions
    ? "You do not have permission to manage invitations"
    : undefined;
  return (
    <div className="flex items-center justify-between gap-4 border-b py-2 last:border-b-0">
      <div className="flex flex-col">
        <div className="text-sm text-content-secondary">{invite.email}</div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {invite.expired && (
          <span className="text-sm font-semibold text-content-error">
            Invitation expired
          </span>
        )}
        <RoleDisplay role={invite.role} customRoles={invite.customRoles} />
        <Menu
          placement="bottom-end"
          buttonProps={{
            variant: "neutral",
            size: "xs",
            icon: <DotsVerticalIcon />,
            "aria-label": "Invitation options",
          }}
        >
          <MenuItem
            disabled={!hasAdminPermissions}
            tip={noPermissionTip}
            tipSide="left"
            action={onResend}
          >
            Resend
          </MenuItem>
          <MenuItem
            variant="danger"
            disabled={!hasAdminPermissions}
            tip={noPermissionTip}
            tipSide="left"
            action={() => onCancelInvite({ email: invite.email })}
          >
            Revoke
          </MenuItem>
        </Menu>
      </div>
    </div>
  );
}
