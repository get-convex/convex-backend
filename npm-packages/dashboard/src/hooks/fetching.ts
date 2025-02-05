import { Middleware } from "swr";
import { captureException, captureMessage } from "@sentry/nextjs";
import { deploymentFetch, translateResponse } from "dashboard-common";
import { useAccessToken } from "./useServerSideData";

export function useAuthHeader() {
  const [accessToken] = useAccessToken();
  if (!accessToken) {
    throw new Error("Attempted to retrieve auth header without access token");
  }
  return `Bearer ${accessToken}`;
}

export function getGoogleAnalyticsClientId(documentCookie: string) {
  try {
    const cookie: { [key: string]: string | undefined } = {};
    documentCookie.split(";").forEach((el) => {
      const splitCookie = el.split("=");
      const key = splitCookie[0].trim();
      const value = splitCookie[1];
      cookie[key] = value;
    });
    // The first 6 characters of the GA cookie should be ignored.
    return cookie._ga?.substring(6) ?? "";
  } catch (e) {
    captureException(e);
    return "";
  }
}

// enabled globally by context
export async function fetchWithAuthHeader([url, authHeader]: [
  url: string,
  authHeader: string,
]): Promise<any> {
  const googleAnalyticsId = getGoogleAnalyticsClientId(document.cookie);
  // Don't transform normal fetch errors.
  try {
    const resp = await fetch(`${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}${url}`, {
      headers: {
        Authorization: authHeader,
        "Convex-Client": "dashboard-0.0.0",
        "Google-Analytics-Client-Id": googleAnalyticsId,
      },
      mode: "cors",
    });
    return await translateResponse(resp);
  } catch (e) {
    if (e instanceof TypeError) {
      // TypeError is thrown when a network error occurs.
      // Often, this is due to the user losing internet connection, or
      // the request being canceled due to page navigation.
      // So, return nothing and allow SWR to retry.
      return;
    }
    throw e;
  }
}

export const bigBrainAuth: Middleware =
  (useSWRNext) => (key, fetcher, config) => {
    // Handle edge cases:
    // 1. If we're fetching from a deployment, don't authenticate to big brain
    // 2. If the type of the key is a function, we're probably hitting a paginated API,
    //    which doesn't work with bigBrainAuth right now. Paginated API calls should supply
    //    their own fetcher.
    if (fetcher === deploymentFetch || typeof key === "function") {
      return useSWRNext(key, fetcher, config);
    }

    let swrKey = key;

    const authHeader = useAuthHeader();
    if (!key) {
      swrKey = null;
    } else if (fetcher === fetchWithAuthHeader) {
      // Only replace the key if we're using the old-style fetcher.
      swrKey = [key, authHeader];
    }

    let fallbackKey = key;
    // If the key is an array, we're using the new-style fetcher.
    // This fetcher is an array of the form ["big-brain", path, params]
    if (Array.isArray(fallbackKey)) {
      const params = fallbackKey[2]?.params?.path;
      [, fallbackKey] = fallbackKey;

      // For fallback data that contains parameters,
      // we need to interpolate the parameters into the key.
      // Currently, we only SSR data that has the team id and project id in parameters.
      if (typeof fallbackKey === "string" && params) {
        fallbackKey = fallbackKey.replace("{team_id}", params.team_id);
        fallbackKey = fallbackKey.replace("{project_id}", params.project_id);
      }
    }

    const fallbackData =
      config.fallback?.initialData &&
      config.fallback.initialData[fallbackKey as string];

    return useSWRNext(swrKey, fetcher, { ...config, fallbackData });
  };

export const reportHttpError = (
  method: string,
  url: string,
  error: { code: string; message: string },
) => {
  captureMessage(
    `failed to request ${method} ${url}: ${error.code} - ${error.message} `,
  );
};
