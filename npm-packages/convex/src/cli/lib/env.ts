import { chalkStderr } from "chalk";
import * as dotenv from "dotenv";
import { Context } from "../../bundler/context.js";
import {
  logFailure,
  logFinishedStep,
  logMessage,
  logOutput,
} from "../../bundler/log.js";
import { runSystemQuery } from "./run.js";
import { deploymentFetch, logAndHandleFetchError } from "./utils/utils.js";
import { readFromStdin } from "./utils/stdin.js";
import { promptSecret } from "./utils/prompts.js";
import {
  EXPECTED_CONVEX_URL_NAMES,
  EXPECTED_SITE_URL_NAMES,
} from "./envvars.js";
import {
  CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
  CONVEX_DEPLOYMENT_ENV_VAR_NAME,
  CONVEX_SELF_HOSTED_URL_VAR_NAME,
  CONVEX_SELF_HOSTED_ADMIN_KEY_VAR_NAME,
} from "./utils/utils.js";

function formatList(items: string[]): string {
  if (items.length === 0) return "";
  if (items.length === 1) return items[0];
  if (items.length === 2) return `${items[0]} and ${items[1]}`;
  return `${items.slice(0, -1).join(", ")}, and ${items[items.length - 1]}`;
}

export async function envSetInDeployment(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
  },
  originalName: string | undefined,
  originalValue: string | undefined,
  options?: {
    fromFile?: string;
    force?: boolean;
    secret?: boolean;
  },
) {
  const { fromFile, force = false } = options ?? {};
  if (originalName) {
    let name = originalName,
      value: string;
    const parsed = await allowEqualsSyntax(ctx, originalName, originalValue);
    if (parsed) {
      [name, value] = parsed;
    } else if (fromFile) {
      value = await getFileContents(ctx, fromFile);
    } else if (!process.stdin.isTTY) {
      value = await getStdIn(ctx);
    } else {
      value = await promptSecret(ctx, {
        message: `Enter value for ${name}:`,
      });
    }
    await callUpdateEnvironmentVariables(ctx, deployment, [{ name, value }]);
    if (options?.secret) {
      const formatted = /\s/.test(value) ? `"${value}"` : value;
      logFinishedStep(
        `Successfully set ${chalkStderr.bold(name)} to ${chalkStderr.bold(formatted)}${deployment.deploymentNotice}`,
      );
    } else {
      logFinishedStep(`Successfully set ${chalkStderr.bold(name)}`);
    }
    return true;
  }
  let content: string, source: string;
  if (fromFile) {
    content = await getFileContents(ctx, fromFile);
    source = fromFile;
  } else if (!process.stdin.isTTY) {
    content = await getStdIn(ctx);
    source = "stdin";
  } else {
    return false;
  }
  await envSetFromContentInDeployment(ctx, deployment, {
    content,
    source,
    force,
  });
  return true;
}

async function getFileContents(
  ctx: Context,
  filePath: string,
): Promise<string> {
  if (!ctx.fs.exists(filePath)) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `error: file not found: ${filePath}`,
    });
  }
  return ctx.fs.readUtf8File(filePath);
}

async function getStdIn(ctx: Context): Promise<string> {
  try {
    return await readFromStdin();
  } catch (error) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `error: failed to read from stdin: ${error instanceof Error ? error.message : String(error)}`,
    });
  }
}

async function envSetFromContentInDeployment(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
  },
  options: {
    content: string;
    source: string;
    force: boolean;
  },
) {
  const { content, source, force } = options;
  const parsedEnv = dotenv.parse(content);

  // Filter out CLI-managed environment variables
  const envVars = Object.entries(parsedEnv);
  const filteredVars: string[] = [];

  const envVarsToSet: [string, string][] = [];
  const managedVars = new Set<string>([
    CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
    CONVEX_DEPLOYMENT_ENV_VAR_NAME,
    CONVEX_SELF_HOSTED_URL_VAR_NAME,
    CONVEX_SELF_HOSTED_ADMIN_KEY_VAR_NAME,
    ...EXPECTED_CONVEX_URL_NAMES,
    ...EXPECTED_SITE_URL_NAMES,
  ]);
  for (const [name, value] of envVars) {
    if (managedVars.has(name)) {
      filteredVars.push(name);
    } else {
      envVarsToSet.push([name, value]);
    }
  }

  if (filteredVars.length > 0) {
    const varNames = filteredVars.map((n) => chalkStderr.bold(n));
    const formattedNames = formatList(varNames);
    logMessage(
      `Skipping ${filteredVars.length} CLI-managed environment variable${filteredVars.length === 1 ? "" : "s"}: ${formattedNames}`,
    );
  }

  if (envVarsToSet.length === 0) {
    if (envVars.length === 0) {
      logMessage(`No environment variables found in ${source}.`);
    }
    return;
  }

  // Fetch existing environment variables
  const existingEnvVars = await getEnvVars(ctx, deployment);

  const existingEnvMap = new Map(
    existingEnvVars.map((env) => [env.name, env.value]),
  );

  // Categorize the environment variables
  const newVars: [string, string][] = [];
  const updatedVars: [string, string][] = [];
  const unchangedVars: [string, string][] = [];
  const conflicts: { name: string; existing: string; new: string }[] = [];

  for (const [name, value] of envVarsToSet) {
    const existingValue = existingEnvMap.get(name);
    if (existingValue === undefined) {
      newVars.push([name, value]);
    } else if (existingValue === value) {
      unchangedVars.push([name, value]);
    } else if (force) {
      updatedVars.push([name, value]);
    } else {
      conflicts.push({ name, existing: existingValue, new: value });
    }
  }

  // Check for conflicts if not replacing
  if (conflicts.length > 0) {
    const varNames = conflicts.map((c) => chalkStderr.bold(c.name));
    const formattedNames = formatList(varNames);
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        `error: environment variable${conflicts.length === 1 ? "" : "s"} ${formattedNames} already exist${conflicts.length === 1 ? "s" : ""} with different value${conflicts.length === 1 ? "" : "s"}.\n\n` +
        `Use ${chalkStderr.bold("--force")} to overwrite existing values.`,
    });
  }

  // Build the changes: only new vars when not replacing, new + updated when replacing
  const varsToUpdate = force ? [...newVars, ...updatedVars] : newVars;
  const changes: EnvVarChange[] = varsToUpdate.map(([name, value]) => ({
    name,
    value,
  }));

  if (changes.length > 0) {
    await callUpdateEnvironmentVariables(ctx, deployment, changes);
  }

  const newCount = newVars.length;
  const updatedCount = updatedVars.length;
  const unchangedCount = unchangedVars.length;

  const parts = [];
  if (newCount > 0) parts.push(`${newCount} new`);
  if (updatedCount > 0) parts.push(`${updatedCount} updated`);
  if (unchangedCount > 0) parts.push(`${unchangedCount} unchanged`);

  const totalProcessed = newCount + updatedCount + unchangedCount;
  if (changes.length === 0) {
    logMessage(
      `All ${totalProcessed} environment variable${totalProcessed === 1 ? "" : "s"} from ${chalkStderr.bold(source)} already set${deployment.deploymentNotice}`,
    );
  } else {
    logFinishedStep(
      `Successfully set ${changes.length} environment variable${changes.length === 1 ? "" : "s"} from ${chalkStderr.bold(source)} (${parts.join(", ")})${deployment.deploymentNotice}`,
    );
  }
}

async function allowEqualsSyntax(
  ctx: Context,
  name: string,
  value: string | undefined,
): Promise<[string, string] | null> {
  if (/^[a-zA-Z][a-zA-Z0-9_]*=/.test(name)) {
    const [n, ...values] = name.split("=");
    if (value === undefined) {
      return [n, values.join("=")];
    } else {
      await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `When setting an environment variable, you can either set a value with 'NAME=value', or with NAME value, but not both. Are you missing quotes around the CLI argument? Try: \n  npx convex env set '${name} ${value}'`,
      });
    }
  }
  if (value === undefined) return null;
  return [name, value];
}

export async function envGetInDeploymentAction(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
  },
  name: string,
) {
  const envVar = await envGetInDeployment(ctx, deployment, name);
  if (envVar === null) {
    logFailure(`Environment variable "${name}" not found.`);
    return;
  }
  logOutput(`${envVar}`);
}

export async function envGetInDeployment(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
  },
  name: string,
): Promise<string | null> {
  const envVar = (await runSystemQuery(ctx, {
    ...deployment,
    functionName: "_system/cli/queryEnvironmentVariables:get",
    componentPath: undefined,
    args: { name },
  })) as EnvVar | null;
  return envVar === null ? null : envVar.value;
}

export async function envRemoveInDeployment(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
  },
  name: string,
) {
  await callUpdateEnvironmentVariables(ctx, deployment, [{ name }]);
  logFinishedStep(
    `Successfully unset ${chalkStderr.bold(name)}${deployment.deploymentNotice}`,
  );
}

async function getEnvVars(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
  },
): Promise<EnvVar[]> {
  return (await runSystemQuery(ctx, {
    ...deployment,
    functionName: "_system/cli/queryEnvironmentVariables",
    componentPath: undefined,
    args: {},
  })) as EnvVar[];
}

export async function envListInDeployment(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
  },
) {
  const envs = await getEnvVars(ctx, deployment);
  if (envs.length === 0) {
    logMessage("No environment variables set.");
    return;
  }
  for (const { name, value } of envs) {
    const { formatted, warning } = formatEnvValueForDotfile(value);
    if (warning) {
      logMessage(`Warning (${name}): ${warning}`);
    }
    logOutput(`${name}=${formatted}`);
  }
}

export function formatEnvValueForDotfile(value: string): {
  formatted: string;
  warning: string | undefined;
} {
  let formatted = value,
    warning: string | undefined = undefined;

  const containsNewline = value.includes("\n");
  const containsSingleQuote = value.includes("'");
  const containsDoubleQuote = value.includes('"');
  const containsSlashN = value.includes("\\n");
  const commentWarning =
    value.includes("#") &&
    `includes a '#' which may be interpreted as a comment if you save this value to a .env file, resulting in only reading a partial value.`;
  if (containsNewline) {
    if (!containsSingleQuote) {
      formatted = `'${value}'`;
    } else if (!containsSlashN) {
      if (containsDoubleQuote && commentWarning) {
        warning = commentWarning;
      }
      formatted = `"${value.replaceAll("\n", "\\n")}"`;
    } else {
      formatted = `'${value}'`;
      warning = `includes single quotes, newlines and "\\n" in the value. If you save this value to a .env file, it may not round-trip.`;
    }
  } else if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'")) ||
    value.startsWith("`") ||
    value.endsWith("`") ||
    value.includes("\f") ||
    value.includes("\v") ||
    commentWarning
  ) {
    if (containsSingleQuote && !containsDoubleQuote && !containsSlashN) {
      formatted = `"${value}"`;
    } else {
      formatted = `'${value}'`;
      if (containsSingleQuote && commentWarning) {
        warning = commentWarning;
      }
    }
  }
  if (value.includes("\r")) {
    warning = warning ? `${warning} It also ` : "";
    warning += `includes carriage return (\\r) which cannot be preserved in .env files (dotenv limitation)`;
  }

  return { formatted, warning };
}

export type EnvVarChange = {
  name: string;
  value?: string;
};

export type EnvVar = {
  name: string;
  value: string;
};

export async function callUpdateEnvironmentVariables(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
  },
  changes: EnvVarChange[],
) {
  const fetch = deploymentFetch(ctx, deployment);
  try {
    await fetch("/api/update_environment_variables", {
      body: JSON.stringify({ changes }),
      method: "POST",
    });
  } catch (e) {
    return await logAndHandleFetchError(ctx, e);
  }
}

export async function fetchDeploymentCanonicalSiteUrl(
  ctx: Context,
  options: { deploymentUrl: string; adminKey: string },
): Promise<string> {
  const result = await envGetInDeployment(ctx, options, "CONVEX_SITE_URL");
  if (typeof result !== "string") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem or env vars",
      printedMessage: "Invalid process.env.CONVEX_SITE_URL",
    });
  }
  return result;
}
