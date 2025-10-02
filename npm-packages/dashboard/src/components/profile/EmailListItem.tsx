import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Menu, MenuItem } from "@ui/Menu";
import {
  useDeleteProfileEmail,
  useIdentities,
  useResendProfileEmailVerification,
  useUpdatePrimaryProfileEmail,
} from "api/profile";
import { useState } from "react";
import { MemberEmailResponse } from "generatedApi";

export function EmailListItem({ email }: { email: MemberEmailResponse }) {
  const identities = useIdentities();
  const emailIsAnIdentity = identities?.some(
    (identity) => identity.email === email.email,
  );
  const deleteEmail = useDeleteProfileEmail();
  const updatePrimaryEmail = useUpdatePrimaryProfileEmail();
  const resentEmailVerification = useResendProfileEmailVerification();

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
          disabled={!email.isVerified || email.isPrimary}
          tip={
            !email.isVerified
              ? "This email is not verified."
              : email.isPrimary
                ? "This is already your primary email."
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
          disabled={email.isPrimary || emailIsAnIdentity}
          variant="danger"
          tip={
            email.isPrimary
              ? "You cannot delete your primary email."
              : emailIsAnIdentity
                ? "You cannot delete this email because it is associated with an identity on your account. Delete the identity first to remove this email from your account."
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
            <div className="flex flex-col gap-1">
              <p>Deleting this email will remove it from your account.</p>
              <p>
                Note: If you login again later with a connected identity
                associated with this email, this email will be re-added to your
                account.
              </p>
            </div>
          }
          error={error}
          validationText={email.email}
        />
      )}
    </div>
  );
}
