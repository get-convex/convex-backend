/* SWR handles caching and retries for us but we have to configure it.
 * This file
 * - defines fetchers: functions that makes the API call and transforms responses
 * - defines middleware: functions that can use react hooks to gather credentials
 *
 * Middleware is set globally and only activates when used with the appropriate fetcher.
 */

class ServerError extends Error {
  status: number;

  statusText: string;

  code: string;

  serverMessage: string;

  constructor(
    message: string,
    resp: Response,
    data: { code: string; message: string },
  ) {
    super(message);
    this.name = "ServerError";
    this.code = data.code;
    this.serverMessage = data.message;
    this.status = resp.status;
    this.statusText = resp.statusText;
  }
}

export async function translateResponse(resp: Response) {
  if (!resp.ok) {
    // First, look for a ServerError with a code.
    let json;
    try {
      json = await resp.json();
    } catch (e) {
      // Failed fetches often don't return JSON.
      if (e instanceof SyntaxError) {
        throw new Error(
          `Server responded with ${resp.status} ${resp.statusText}`,
        );
      }
      throw e;
    }
    // Next, look for an error from the server with an error code.
    if ("code" in json) {
      // If the error is a 401 - not authorized, redirect to the login page.
      if (resp.status === 401) {
        if (window.location.pathname !== "/login") {
          // Ideally we'd refresh the token here, but because we're issuing a ton
          // of requests concurrently it gets really tricky,
          // so instead we proactively refresh the token in _app.tsx,
          // so we should never get here unless the user genuinely lost access.
          window.location.assign(`/login?returnTo=${window.location.pathname}`);
        }
        return;
      }
      throw new ServerError(
        `Server responded with ${resp.status}: ${json.code} ${json.message}`,
        resp,
        json,
      );
    }
    // This doesn't look like our error, but there's still valid JSON.
    throw new Error(
      `Server responded with ${resp.status} ${resp.statusText} ${JSON.stringify(
        json,
      ).slice(0, 1000)}`,
    );
  } else {
    return resp.json();
  }
}

function asThreeTupleOfStrings(
  args: readonly unknown[],
): [a: string, b: string, c: string] {
  if (!Array.isArray(args)) {
    throw new Error("Fetcher arg is not an array");
  }
  if (args.length < 3) {
    throw new Error("Fetcher args array too short");
  }
  if (args.some((x) => typeof x !== "string")) {
    throw new Error("Fetcher arg element is not a string");
  }
  return args as [string, string, string];
}

// Expects an array of [deploymentUrl, path, authHeader] but the types don't work out to write it like that.
export async function deploymentFetch(
  args: readonly [deploymentUrl: string, ...rest: unknown[]],
): Promise<any> {
  const [deploymentUrl, path, authHeader] = asThreeTupleOfStrings(args);
  // Don't transform normal fetch errors.
  const url = deploymentUrl + path;
  try {
    const resp = await fetch(url, {
      headers: {
        Authorization: authHeader,
        "Convex-Client": "dashboard-0.0.0",
      },
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
