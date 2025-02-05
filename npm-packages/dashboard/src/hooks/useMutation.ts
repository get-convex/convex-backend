import { toast } from "dashboard-common";
import { useCallback } from "react";
import { useSWRConfig } from "swr";
import { reportHttpError, useAuthHeader } from "./fetching";

type MutateOptions = {
  url: string;
  mutateKey?: string;
  successToast?: string;
  toastOnError?: boolean;
};

// Makes a mutative API request, handling errors and toasts.
export function useMutation<Request>({
  url,
  mutateKey,
  successToast,
  toastOnError = true,
}: MutateOptions): (body: Request) => Promise<globalThis.Response> {
  const { mutate } = useSWRConfig();
  const authHeader = useAuthHeader();
  return useCallback(
    async (request: Request) => {
      if (authHeader === null) {
        toast(
          "error",
          "An error occurred authenticating your request. Please try again.",
          "authHeader",
        );
        throw new Error("authHeader error");
      }
      const response = await fetch(
        `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}${url}`,
        {
          method: "POST",
          headers: {
            Authorization: authHeader,
            "Content-Type": "application/json",
          },
          body: JSON.stringify(request),
        },
      );
      if (!response.ok) {
        const error = await response.json();
        reportHttpError("POST", url, error);
        toastOnError && toast("error", error.message, error.message);
        throw error;
      } else {
        mutateKey && (await mutate([mutateKey, authHeader]));
        successToast && toast("success", successToast);
        return response;
      }
    },
    [authHeader, mutate, mutateKey, successToast, toastOnError, url],
  );
}
