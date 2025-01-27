import createClient, {
  RequestBodyOption,
  MaybeOptionalInit,
  HeadersOptions,
  FetchResponse,
} from "openapi-fetch";
import { PathsWithMethod } from "openapi-typescript-helpers";
import { createMutateHook, createQueryHook } from "swr-openapi";
import isMatch from "lodash/isMatch";
import { fireGoogleAnalyticsEvent } from "elements/GoogleAnalytics";
import { reportHttpError, toast } from "dashboard-common";
import type { paths as BigBrainPaths } from "generatedApi";
import { SWRConfiguration } from "swr";
import { useAccessToken } from "hooks/useServerSideData";
import { useRouter } from "next/router";
import { useCallback, useEffect } from "react";
import { usePrevious } from "react-use";
import { getGoogleAnalyticsClientId } from "../hooks/fetching";

export const client = createClient<BigBrainPaths>({
  baseUrl: `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard`,
});

const useQuery = createQueryHook(client, "big-brain");

export const useMutate = createMutateHook(client, "big-brain", isMatch);

type Path<M extends "post" | "put" | "get"> = PathsWithMethod<BigBrainPaths, M>;

export function useBBQuery<QueryPath extends Path<"get">>(
  path: QueryPath,
  pathParams: BigBrainPaths[QueryPath]["get"]["parameters"]["path"],
  swrOptions?: SWRConfiguration,
) {
  const googleAnalyticsId =
    typeof document !== "undefined" &&
    getGoogleAnalyticsClientId(document.cookie);
  const [accessToken] = useAccessToken();
  const authHeader = `Bearer ${accessToken}`;

  const headers: HeadersOptions = {
    Authorization: authHeader,
    "Convex-Client": "dashboard-0.0.0",
    "Google-Analytics-Client-Id": googleAnalyticsId,
  };

  // @ts-expect-error TODO: Figure out how to type this.
  const requestOptions: MaybeOptionalInit<BigBrainPaths[QueryPath], "get"> = {
    params: { path: pathParams },
    headers,
  };
  const paused =
    !accessToken || (pathParams && Object.values(pathParams).some((p) => !p));
  const previousPaused = usePrevious(paused);
  const mutate = useMutate();

  useEffect(() => {
    previousPaused &&
      !paused &&
      void mutate(
        [
          path,
          // @ts-expect-error TODO: Figure out how to type this.
          { params: { path: pathParams } },
        ],
        undefined,
      );
  }, [paused, mutate, path, pathParams, previousPaused]);

  return useQuery(path, requestOptions, {
    keepPreviousData: true,
    isPaused: () => paused,
    ...swrOptions,
  });
}

// Makes a mutative API request, handling errors and toasts.
export function useBBMutation<
  T extends Path<Method>,
  M extends Path<"get">,
  Method extends "post" | "put" = "post",
>({
  path,
  pathParams,
  successToast,
  toastOnError = true,
  googleAnalyticsEvent,
  redirectTo,
  method = "post" as Method,
  ...rest
}: {
  path: T;
  method?: Method;
  pathParams: BigBrainPaths[T][Method] extends { parameters: {} }
    ? BigBrainPaths[T][Method]["parameters"]["path"]
    : undefined;
  successToast?: string;
  toastOnError?: boolean;
  redirectTo?: string;
  googleAnalyticsEvent?: string;
} & (
  | {
      mutateKey: M;
      mutatePathParams: BigBrainPaths[M]["get"]["parameters"]["path"];
    }
  | {}
)) {
  const router = useRouter();
  const [accessToken] = useAccessToken();
  const authHeader = `Bearer ${accessToken}`;
  const mutate = useMutate();
  const googleAnalyticsId = getGoogleAnalyticsClientId(document.cookie);

  return useCallback(
    async (
      ...body: RequestBodyOption<
        BigBrainPaths[T][Method]
      >["body"] extends undefined
        ? []
        : [RequestBodyOption<BigBrainPaths[T][Method]>["body"]]
    ) => {
      if (!authHeader) {
        toast(
          "error",
          "An error occurred authenticating your request. Please try again.",
          "authHeader",
        );
        throw new Error("authHeader error");
      }

      const headers: HeadersOptions = {
        Authorization: authHeader,
        "Convex-Client": "dashboard-0.0.0",
        "Google-Analytics-Client-Id": googleAnalyticsId,
      };

      const call = method === "put" ? client.PUT : client.POST;
      const {
        error,
        data,
        response,
      }: FetchResponse<BigBrainPaths[T], any, "application/json"> =
        // @ts-ignore TODO: Figure out how to type this.
        await call(path, {
          params: { path: pathParams },
          body: body[0],
          headers,
        });

      if (!response.ok && error) {
        reportHttpError(
          "POST",
          path,
          error as unknown as { message: string; code: string },
        );
        // @ts-expect-error Errors are not yet defined in the OpenAPI spec.
        toastOnError && toast("error", error.message, error.message);
        // eslint-disable-next-line @typescript-eslint/no-throw-literal
        throw error;
      }
      redirectTo && (await router.push(redirectTo));
      if ("mutateKey" in rest) {
        const { mutateKey, mutatePathParams } = rest;
        await mutate(
          [
            mutateKey,
            // @ts-expect-error TODO: Figure out how to type this.
            { params: { path: mutatePathParams } },
          ],
          undefined,
        );
      }
      successToast && toast("success", successToast);
      googleAnalyticsEvent && fireGoogleAnalyticsEvent(googleAnalyticsEvent);
      return data;
    },
    [
      authHeader,
      googleAnalyticsEvent,
      googleAnalyticsId,
      method,
      mutate,
      path,
      pathParams,
      redirectTo,
      rest,
      router,
      successToast,
      toastOnError,
    ],
  );
}
