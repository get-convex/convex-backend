import { useBBMutation, useBBQuery } from "./api";

export function useProfile() {
  const { data: profile } = useBBQuery({
    path: "/profile",
    pathParams: undefined,
  });
  return profile;
}

export function useUpdateProfileName() {
  return useBBMutation({
    path: "/update_profile_name",
    pathParams: undefined,
    mutateKey: "/profile",
    successToast: "Name updated.",
    method: "put",
  });
}

export function useProfileEmails() {
  const { data: emails } = useBBQuery({
    path: "/profile_emails/list",
    pathParams: undefined,
  });
  return emails;
}

export function useCreateProfileEmail() {
  return useBBMutation({
    path: "/profile_emails/create",
    pathParams: undefined,
    mutateKey: "/profile_emails/list",
    successToast: "Verification email sent.",
    toastOnError: false,
  });
}

export function useDeleteProfileEmail() {
  return useBBMutation({
    path: "/profile_emails/delete",
    pathParams: undefined,
    mutateKey: `/profile_emails/list`,
    successToast: "Email removed.",
    toastOnError: false,
  });
}

export function useUpdatePrimaryProfileEmail() {
  return useBBMutation({
    path: "/profile_emails/update_primary",
    pathParams: undefined,
    mutateKey: `/profile_emails/list`,
    successToast: "Primary email updated.",
  });
}

export function useResendProfileEmailVerification() {
  return useBBMutation({
    path: `/profile_emails/resend_verification`,
    pathParams: undefined,
    successToast: "Verification email sent.",
  });
}

export function useVerifyProfileEmail(code: string) {
  return useBBMutation({
    path: "/profile_emails/verify/{code}",
    pathParams: { code },
    mutateKey: `/profile_emails/list`,
    successToast: "Email verified.",
    toastOnError: false,
    redirectTo: "/profile",
  });
}

export function useDeleteAccount() {
  return useBBMutation({
    path: "/delete_account",
    pathParams: undefined,
    successToast: "Account deleted.",
    toastOnError: false,
  });
}

export function useIdentities() {
  const { data: identities } = useBBQuery({
    path: "/identities",
    pathParams: undefined,
  });
  return identities;
}

export function useUnlinkIdentity() {
  return useBBMutation({
    path: "/unlink_identity",
    pathParams: undefined,
    mutateKey: "/identities",
    successToast: "Identity unlinked.",
    toastOnError: false,
  });
}
