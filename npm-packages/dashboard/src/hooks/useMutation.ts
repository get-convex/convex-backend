import { reportHttpError } from "lib/utils";
import { toast } from "dashboard-common";
import { useCallback } from "react";
import { useSWRConfig } from "swr";
import { fireGoogleAnalyticsEvent } from "elements/GoogleAnalytics";
import { useRouter } from "next/router";
import { useAuthHeader } from "./fetching";

type MutateOptions = {
  url: string;
  mutateKey?: string;
  successToast?: string;
  method?: "POST" | "PUT";
  toastOnError?: boolean;
  redirectTo?: string;
  googleAnalyticsEvent?: string;
};

// Makes a mutative API request, handling errors and toasts.
export function useMutation<Request>({
  url,
  method = "POST",
  mutateKey,
  successToast,
  toastOnError = true,
  googleAnalyticsEvent,
  redirectTo,
}: MutateOptions): (body: Request) => Promise<globalThis.Response> {
  const router = useRouter();
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
          method,
          headers: {
            Authorization: authHeader,
            "Content-Type": "application/json",
          },
          body: JSON.stringify(request),
        },
      );
      if (!response.ok) {
        const error = await response.json();
        reportHttpError(method, url, error);
        toastOnError && toast("error", error.message, error.message);
        throw error;
      } else {
        redirectTo && (await router.push(redirectTo));
        mutateKey && (await mutate([mutateKey, authHeader]));
        successToast && toast("success", successToast);
        googleAnalyticsEvent && fireGoogleAnalyticsEvent(googleAnalyticsEvent);
        return response;
      }
    },
    [
      authHeader,
      googleAnalyticsEvent,
      method,
      mutate,
      mutateKey,
      redirectTo,
      router,
      successToast,
      toastOnError,
      url,
    ],
  );
}

// Convenience wrapper for converting the JSON response to a parameterized `Response` type.
export function useMutationWithResponse<Request, Response>(
  options: MutateOptions,
): (body: Request) => Promise<Response> {
  const mutate = useMutation<Request>(options);
  return useCallback(
    async (request: Request): Promise<Response> =>
      (await mutate(request)).json(),
    [mutate],
  );
}
