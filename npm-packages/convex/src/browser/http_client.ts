import {
  FunctionReference,
  FunctionReturnType,
  OptionalRestArgs,
  getFunctionName,
} from "../server/api.js";
import { parseArgs, validateDeploymentUrl } from "../common/index.js";
import { version } from "../index.js";
import {
  ConvexError,
  JSONValue,
  convexToJson,
  jsonToConvex,
} from "../values/index.js";
import { logToConsole } from "./logging.js";
import { UserIdentityAttributes } from "../server/index.js";

export const STATUS_CODE_OK = 200;
export const STATUS_CODE_BAD_REQUEST = 400;
// Special custom 5xx HTTP status code to mean that the UDF returned an error.
//
// Must match the constant of the same name in the backend.
export const STATUS_CODE_UDF_FAILED = 560;

// Allow fetch to be shimmed in for Node.js < 18
let specifiedFetch: typeof globalThis.fetch | undefined = undefined;
export function setFetch(f: typeof globalThis.fetch) {
  specifiedFetch = f;
}

/**
 * A Convex client that runs queries and mutations over HTTP.
 *
 * This is appropriate for server-side code (like Netlify Lambdas) or non-reactive
 * webapps.
 *
 * If you're building a React app, consider using
 * {@link react.ConvexReactClient} instead.
 *
 * @public
 */
export class ConvexHttpClient {
  private readonly address: string;
  private auth?: string;
  private adminAuth?: string;
  private debug: boolean;
  private fetchOptions?: FetchOptions;

  /**
   * Create a new {@link ConvexHttpClient}.
   *
   * @param address - The url of your Convex deployment, often provided
   * by an environment variable. E.g. `https://small-mouse-123.convex.cloud`.
   * @param skipConvexDeploymentUrlCheck - Skip validating that the Convex deployment URL looks like
   * `https://happy-animal-123.convex.cloud` or localhost. This can be useful if running a self-hosted
   * Convex backend that uses a different URL.
   */
  constructor(address: string, skipConvexDeploymentUrlCheck?: boolean) {
    if (skipConvexDeploymentUrlCheck !== true) {
      validateDeploymentUrl(address);
    }
    this.address = `${address}/api`;
    this.debug = true;
  }

  /**
   * Obtain the {@link ConvexHttpClient}'s URL to its backend.
   *
   * @returns The URL to the Convex backend, including the client's API version.
   */
  backendUrl(): string {
    return this.address;
  }

  /**
   * Set the authentication token to be used for subsequent queries and mutations.
   *
   * Should be called whenever the token changes (i.e. due to expiration and refresh).
   *
   * @param value - JWT-encoded OpenID Connect identity token.
   */
  setAuth(value: string) {
    this.clearAuth();
    this.auth = value;
  }

  /**
   * @internal
   */
  setAdminAuth(token: string, actingAsIdentity?: UserIdentityAttributes) {
    this.clearAuth();
    if (actingAsIdentity !== undefined) {
      // Encode the identity to a base64 string
      const bytes = new TextEncoder().encode(JSON.stringify(actingAsIdentity));
      const actingAsIdentityEncoded = btoa(String.fromCodePoint(...bytes));
      this.adminAuth = `${token}:${actingAsIdentityEncoded}`;
    } else {
      this.adminAuth = token;
    }
  }

  /**
   * Clear the current authentication token if set.
   */
  clearAuth() {
    this.auth = undefined;
    this.adminAuth = undefined;
  }

  /**
   * Sets whether the result log lines should be printed on the console or not.
   *
   * @internal
   */
  setDebug(debug: boolean) {
    this.debug = debug;
  }

  /**
   * Used to customize the fetch behavior in some runtimes.
   *
   * @internal
   */
  setFetchOptions(fetchOptions: FetchOptions) {
    this.fetchOptions = fetchOptions;
  }

  /**
   * Execute a Convex query function.
   *
   * @param name - The name of the query.
   * @param args - The arguments object for the query. If this is omitted,
   * the arguments will be `{}`.
   * @returns A promise of the query's result.
   */
  async query<Query extends FunctionReference<"query">>(
    query: Query,
    ...args: OptionalRestArgs<Query>
  ): Promise<FunctionReturnType<Query>> {
    const queryArgs = parseArgs(args[0]);
    const name = getFunctionName(query);
    const body = JSON.stringify({
      path: name,
      format: "convex_encoded_json",
      args: [convexToJson(queryArgs)],
    });
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "Convex-Client": `npm-${version}`,
    };
    if (this.adminAuth) {
      headers["Authorization"] = `Convex ${this.adminAuth}`;
    } else if (this.auth) {
      headers["Authorization"] = `Bearer ${this.auth}`;
    }
    const localFetch = specifiedFetch || fetch;
    const response = await localFetch(`${this.address}/query`, {
      ...this.fetchOptions,
      body,
      method: "POST",
      headers: headers,
      credentials: "include",
    });
    if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
      throw new Error(await response.text());
    }
    const respJSON = await response.json();

    if (this.debug) {
      for (const line of respJSON.logLines ?? []) {
        logToConsole("info", "query", name, line);
      }
    }
    switch (respJSON.status) {
      case "success":
        return jsonToConvex(respJSON.value);
      case "error":
        if (respJSON.errorData !== undefined) {
          throw forwardErrorData(
            respJSON.errorData,
            new ConvexError(respJSON.errorMessage),
          );
        }
        throw new Error(respJSON.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(respJSON)}`);
    }
  }

  /**
   * Execute a Convex mutation function.
   *
   * @param name - The name of the mutation.
   * @param args - The arguments object for the mutation. If this is omitted,
   * the arguments will be `{}`.
   * @returns A promise of the mutation's result.
   */
  async mutation<Mutation extends FunctionReference<"mutation">>(
    mutation: Mutation,
    ...args: OptionalRestArgs<Mutation>
  ): Promise<FunctionReturnType<Mutation>> {
    const mutationArgs = parseArgs(args[0]);
    const name = getFunctionName(mutation);
    const body = JSON.stringify({
      path: name,
      format: "convex_encoded_json",
      args: [convexToJson(mutationArgs)],
    });
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "Convex-Client": `npm-${version}`,
    };
    if (this.adminAuth) {
      headers["Authorization"] = `Convex ${this.adminAuth}`;
    } else if (this.auth) {
      headers["Authorization"] = `Bearer ${this.auth}`;
    }
    const localFetch = specifiedFetch || fetch;
    const response = await localFetch(`${this.address}/mutation`, {
      ...this.fetchOptions,
      body,
      method: "POST",
      headers: headers,
      credentials: "include",
    });
    if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
      throw new Error(await response.text());
    }
    const respJSON = await response.json();
    if (this.debug) {
      for (const line of respJSON.logLines ?? []) {
        logToConsole("info", "mutation", name, line);
      }
    }
    switch (respJSON.status) {
      case "success":
        return jsonToConvex(respJSON.value);
      case "error":
        if (respJSON.errorData !== undefined) {
          throw forwardErrorData(
            respJSON.errorData,
            new ConvexError(respJSON.errorMessage),
          );
        }
        throw new Error(respJSON.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(respJSON)}`);
    }
  }

  /**
   * Execute a Convex action function.
   *
   * @param name - The name of the action.
   * @param args - The arguments object for the action. If this is omitted,
   * the arguments will be `{}`.
   * @returns A promise of the action's result.
   */
  async action<Action extends FunctionReference<"action">>(
    action: Action,
    ...args: OptionalRestArgs<Action>
  ): Promise<FunctionReturnType<Action>> {
    const actionArgs = parseArgs(args[0]);
    const name = getFunctionName(action);
    const body = JSON.stringify({
      path: name,
      format: "convex_encoded_json",
      args: [convexToJson(actionArgs)],
    });
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "Convex-Client": `npm-${version}`,
    };
    if (this.adminAuth) {
      headers["Authorization"] = `Convex ${this.adminAuth}`;
    } else if (this.auth) {
      headers["Authorization"] = `Bearer ${this.auth}`;
    }
    const localFetch = specifiedFetch || fetch;
    const response = await localFetch(`${this.address}/action`, {
      ...this.fetchOptions,
      body,
      method: "POST",
      headers: headers,
      credentials: "include",
    });
    if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
      throw new Error(await response.text());
    }
    const respJSON = await response.json();
    if (this.debug) {
      for (const line of respJSON.logLines ?? []) {
        logToConsole("info", "action", name, line);
      }
    }
    switch (respJSON.status) {
      case "success":
        return jsonToConvex(respJSON.value);
      case "error":
        if (respJSON.errorData !== undefined) {
          throw forwardErrorData(
            respJSON.errorData,
            new ConvexError(respJSON.errorMessage),
          );
        }
        throw new Error(respJSON.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(respJSON)}`);
    }
  }

  /**
   * Execute a Convex function of an unknown type.
   *
   * @param name - The name of the function.
   * @param args - The arguments object for the function. If this is omitted,
   * the arguments will be `{}`.
   * @returns A promise of the function's result.
   *
   * @internal
   */
  async function<
    AnyFunction extends FunctionReference<"query" | "mutation" | "action">,
  >(
    anyFunction: AnyFunction | string,
    ...args: OptionalRestArgs<AnyFunction>
  ): Promise<FunctionReturnType<AnyFunction>> {
    const functionArgs = parseArgs(args[0]);
    const name =
      typeof anyFunction === "string"
        ? anyFunction
        : getFunctionName(anyFunction);
    const body = JSON.stringify({
      path: name,
      format: "convex_encoded_json",
      args: convexToJson(functionArgs),
    });
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "Convex-Client": `npm-${version}`,
    };
    if (this.adminAuth) {
      headers["Authorization"] = `Convex ${this.adminAuth}`;
    } else if (this.auth) {
      headers["Authorization"] = `Bearer ${this.auth}`;
    }
    const localFetch = specifiedFetch || fetch;
    const response = await localFetch(`${this.address}/function`, {
      ...this.fetchOptions,
      body,
      method: "POST",
      headers: headers,
      credentials: "include",
    });
    if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
      throw new Error(await response.text());
    }
    const respJSON = await response.json();
    if (this.debug) {
      for (const line of respJSON.logLines ?? []) {
        logToConsole("info", "any", name, line);
      }
    }
    switch (respJSON.status) {
      case "success":
        return jsonToConvex(respJSON.value);
      case "error":
        if (respJSON.errorData !== undefined) {
          throw forwardErrorData(
            respJSON.errorData,
            new ConvexError(respJSON.errorMessage),
          );
        }
        throw new Error(respJSON.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(respJSON)}`);
    }
  }
}

function forwardErrorData(errorData: JSONValue, error: ConvexError<string>) {
  (error as ConvexError<any>).data = jsonToConvex(errorData);
  return error;
}

/**
 * @internal
 */
type FetchOptions = { cache: "force-cache" | "no-store" };
