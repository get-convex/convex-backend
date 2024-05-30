import { changeSpinner, Context, logFailure } from "../../bundler/context.js";
import { version } from "../version.js";
import { deploymentFetch, logAndHandleFetchError } from "./utils.js";
import { Bundle } from "../../bundler/index.js";

/** Push configuration2 to the given remote origin. */

export async function startPush(
  ctx: Context,
  adminKey: string,
  url: string,
  functions: string,
  udfServerVersion: string,
  appDefinition: AppDefinitionSpec,
  componentDefinitions: ComponentDefinitionSpec[],
): Promise<StartPushResponse> {
  const serializedConfig = config2JSON(
    adminKey,
    functions,
    udfServerVersion,
    appDefinition,
    componentDefinitions,
  );
  const custom = (_k: string | number, s: any) =>
    typeof s === "string" ? s.slice(0, 40) + (s.length > 40 ? "..." : "") : s;
  console.log(JSON.stringify(serializedConfig, custom, 2));
  const fetch = deploymentFetch(url);
  changeSpinner(ctx, "Analyzing and deploying source code...");
  try {
    const response = await fetch("/api/deploy2/start_push", {
      body: JSON.stringify(serializedConfig),
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Convex-Client": `npm-cli-${version}`,
      },
    });
    return await response.json();
  } catch (error: unknown) {
    // TODO incorporate AuthConfigMissingEnvironmentVariable logic
    logFailure(ctx, "Error: Unable to start push to " + url);
    return await logAndHandleFetchError(ctx, error);
  }
}

export async function finishPush(
  ctx: Context,
  adminKey: string,
  url: string,
  startPush: StartPushResponse,
): Promise<void> {
  const fetch = deploymentFetch(url);
  changeSpinner(ctx, "Finalizing push...");
  try {
    const response = await fetch("/api/deploy2/finish_push", {
      body: JSON.stringify({
        adminKey,
        startPush,
        dryRun: false,
      }),
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Convex-Client": `npm-cli-${version}`,
      },
    });
    return await response.json();
  } catch (error: unknown) {
    // TODO incorporate AuthConfigMissingEnvironmentVariable logic
    logFailure(ctx, "Error: Unable to finish push to " + url);
    return await logAndHandleFetchError(ctx, error);
  }
}

/** A component spec conains sufficient information to create a
 * bundles of code that must be analyzed.
 *
 * Most paths are relative to the directory of the definitionPath.
 */
export type ComponentDefinitionSpec = {
  /** This path is relative to the app (root component) directory. */
  definitionPath: string;
  /** Dependencies are paths to the directory of the dependency component definition from the app (root component) directory */
  dependencies: string[];

  // All other paths are relative to the directory of the definitionPath above.

  definition: Bundle;
  schema: Bundle;
  functions: Bundle[];
};

export type AppDefinitionSpec = Omit<
  ComponentDefinitionSpec,
  "definitionPath"
> & {
  // Only app (root) component specs contain an auth bundle.
  auth: Bundle | null;
};

export type ComponentDefinitionSpecWithoutImpls = Omit<
  ComponentDefinitionSpec,
  "schema" | "functions"
>;
export type AppDefinitionSpecWithoutImpls = Omit<
  AppDefinitionSpec,
  "schema" | "functions" | "auth"
>;

// TODO repetitive now, but this can do some denormalization if helpful
export function config2JSON(
  adminKey: string,
  functions: string,
  udfServerVersion: string,
  appDefinition: AppDefinitionSpec,
  componentDefinitions: ComponentDefinitionSpec[],
): {
  adminKey: string;
  functions: string;
  udfServerVersion: string;
  appDefinition: AppDefinitionSpec;
  componentDefinitions: ComponentDefinitionSpec[];
  nodeDependencies: [];
} {
  return {
    adminKey,
    functions,
    udfServerVersion,
    appDefinition,
    componentDefinitions,
    nodeDependencies: [],
  };
}

export type StartPushResponse = {
  externalDepsId: null | unknown; // this is a guess
  appPackage: string; // like '9e0fbcbe-b2bc-40a3-9273-6a24896ba8ec',
  componentPackages: Record<string, string> /* like {
    '../../convex_ratelimiter/ratelimiter': '4dab8e49-6e40-47fb-ae5b-f53f58ccd244',
    '../examples/waitlist': 'b2eaba58-d320-4b84-85f1-476af834c17f'
  },*/;
  appAuth: unknown[];
  analysis: Record<
    string,
    {
      definition: {
        path: string; // same as key?
        definitionType: { type: "app" } | unknown;
        childComponents: unknown[];
        exports: unknown;
      };
      schema: { tables: unknown[]; schemaValidation: boolean };
      // really this is "modules"
      functions: Record<
        string,
        {
          functions: unknown[];
          httpRoutes: null | unknown;
          cronSpecs: null | unknown;
          sourceMapped: unknown;
        }
      >;
    }
  >;
};
