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

export function useListIdentities() {
  const { data: identities } = useBBQuery({
    path: "/list_identities",
    pathParams: undefined,
  });
  return identities;
}

export function useSetLinkIdentityCookie() {
  return useBBMutation({
    path: "/set_jwt_cookie",
    pathParams: undefined,
    includeCredentials: true,
  });
}

export function useLinkIdentity() {
  return useBBMutation({
    path: "/link_identity",
    pathParams: undefined,
    mutateKey: "/list_identities",
    includeCredentials: true,
  });
}

export function useUnlinkIdentity() {
  return useBBMutation({
    path: "/unlink_identity",
    pathParams: undefined,
    mutateKey: "/list_identities",
    successToast: "Identity removed.",
  });
}

export function useChangePrimaryIdentity() {
  return useBBMutation({
    path: "/update_primary_identity",
    pathParams: undefined,
    mutateKey: "/list_identities",
    successToast: "Primary identity changed.",
  });
}
