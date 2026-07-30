import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Menu, MenuItem } from "@ui/Menu";
import {
  useDeleteProfileEmail,
  useMfaStatus,
  useResendProfileEmailVerification,
  useUpdatePrimaryProfileEmail,
} from "api/profile";
import { useState } from "react";
import { MemberEmailResponse } from "generatedApi";

export function EmailListItem({ email }: { email: MemberEmailResponse }) {
  const deleteEmail = useDeleteProfileEmail();
  const updatePrimaryEmail = useUpdatePrimaryProfileEmail();
  const resentEmailVerification = useResendProfileEmailVerification();
  // Changing the primary email moves which identity MFA is enforced against, so
  // it's blocked (client- and server-side) while MFA is enabled.
  const mfaEnabled = useMfaStatus()?.enabled ?? false;

  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [error, setError] = useState<string>();

  return (
    <div className="flex flex-wrap items-center justify-between gap-4 border-b py-2 last:border-b-0">
      <div className="flex grow items-center gap-2">
        <div className="grow">{email.email}</div>
        {email.isPrimary && (
          <div className="rounded-sm border p-1 text-xs">Primary</div>
        )}
        <div className="rounded-sm border p-1 text-xs">
          {email.isVerified ? "Verified" : "Unverified"}
        </div>
      </div>
      <Menu
        placement="bottom-end"
        buttonProps={{
          variant: "neutral",
          icon: <DotsVerticalIcon />,
          "aria-label": "Email options",
          size: "xs",
        }}
      >
        <MenuItem
          action={() => updatePrimaryEmail({ email: email.email })}
          disabled={!email.isVerified || email.isPrimary || mfaEnabled}
          tip={
            !email.isVerified
              ? "This email is not verified."
              : email.isPrimary
                ? "This is already your primary email."
                : mfaEnabled
                  ? "Disable multi-factor authentication to change your primary email. You can re-enable it afterward."
                  : undefined
          }
          tipSide="right"
        >
          Set as primary
        </MenuItem>
        {!email.isVerified ? (
          <MenuItem
            action={() => resentEmailVerification({ email: email.email })}
          >
            Resend verification email
          </MenuItem>
        ) : null}
        <MenuItem
          action={() => setShowDeleteConfirmation(true)}
          disabled={email.isPrimary}
          variant="danger"
          tip={
            email.isPrimary
              ? "You cannot delete your primary email."
              : undefined
          }
          tipSide="right"
        >
          Delete
        </MenuItem>
      </Menu>
      {showDeleteConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDeleteConfirmation(false);
            setError(undefined);
          }}
          onConfirm={async () => {
            try {
              await deleteEmail({ email: email.email });
              setShowDeleteConfirmation(false);
            } catch (e: any) {
              setError(e.message);
              throw e;
            }
          }}
          confirmText="Delete"
          variant="danger"
          dialogTitle="Delete Email"
          dialogBody={
            <p>Deleting this email will remove it from your account.</p>
          }
          error={error}
          validationText={email.email}
        />
      )}
    </div>
  );
}
