import { useManagementApiMutation, useManagementApiQuery } from "./api";

export function usePersonalAccessTokens() {
  const { data } = useManagementApiQuery({
    path: "/list_personal_access_tokens",
    pathParams: undefined as never,
  });
  return data;
}

export function useCreatePersonalAccessToken() {
  return useManagementApiMutation({
    path: "/create_personal_access_token",
    pathParams: undefined as never,
    mutateKey: "/list_personal_access_tokens",
    mutatePathParams: undefined as never,
    successToast: "Personal access token created.",
  });
}

export function useDeletePersonalAccessToken() {
  return useManagementApiMutation({
    path: "/delete_personal_access_token",
    pathParams: undefined as never,
    mutateKey: "/list_personal_access_tokens",
    mutatePathParams: undefined as never,
    successToast: "Personal access token deleted.",
  });
}
