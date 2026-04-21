import { execSync } from "child_process";
import { Command, Option } from "@commander-js/extra-typings";
import { Context, oneoffContext } from "../bundler/context.js";
import {
  logFailure,
  logFinishedStep,
  logMessage,
  showSpinner,
} from "../bundler/log.js";
import {
  DeploymentSelection,
  getDeploymentSelection,
  getProjectDetails,
  deploymentNameFromSelection,
} from "./lib/deploymentSelection.js";
import {
  logNoDefaultRegionMessage,
  selectRegion,
  typedBigBrainClient,
  typedPlatformClient,
} from "./lib/utils/utils.js";
import { PlatformProjectDetails } from "@convex-dev/platform/managementApi";
import { getTeamAndProjectFromPreviewAdminKey } from "./lib/deployment.js";
import { saveSelectedDeployment } from "./deploymentSelect.js";
import { promptOptions, promptString } from "./lib/utils/prompts.js";
import { chalkStderr } from "chalk";
import { parseDeploymentSelector } from "./lib/deploymentSelector.js";
import {
  parseExpiration,
  resolveExpiration,
  validateExpiration,
} from "./lib/expiration.js";
import { ensureBackendBinaryDownloaded } from "./lib/localDeployment/download.js";
import {
  loadProjectLocalConfig,
  saveDeploymentConfig,
} from "./lib/localDeployment/filePaths.js";
import {
  chooseLocalBackendPorts,
  LOCAL_BACKEND_INSTANCE_SECRET,
} from "./lib/localDeployment/utils.js";
import { bigBrainStart } from "./lib/localDeployment/bigBrain.js";

const SUPPORTED_TYPES = ["dev", "prod", "preview"] as const;

export const deploymentCreate = new Command("create")
  .summary("Create a new deployment for a project")
  .description(
    "Create a new deployment for a project.\n\n" +
      "  Create a dev deployment and select it:    `npx convex deployment create dev/my-new-feature --type dev --select`\n" +
      "  Create a prod deployment named “staging”: `npx convex deployment create staging --type prod`\n" +
      "  Create a local deployment:                `npx convex deployment create local`\n",
  )
  .argument("[ref]")
  .allowExcessArguments(false)
  .addOption(
    new Option("--type <type>", "Deployment type").choices(SUPPORTED_TYPES),
  )
  .option("--region <region>", "Deployment region")
  .addOption(new Option("--class <class>", "Deployment class").hideHelp())
  .option(
    "--select",
    "Select the new deployment. This will update the Convex environment variables in .env.local. Subsequent `npx convex` commands will run against this deployment.",
  )
  .option(
    "--default",
    "Make the new deployment your default production deployment (used by `npx convex deploy`) or your personal dev deployment.",
  )
  .option(
    "--expiration <value>",
    'When the deployment expires (e.g. "none", "in 7 days", "2026-04-01T00:00:00Z", or a UNIX timestamp in seconds or milliseconds)',
  )
  .action(async (refParam, options) => {
    const ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });

    const currentDeployment = await getDeploymentSelection(ctx, {
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });

    // Handle `deployment create local`
    if (refParam !== undefined) {
      if (refParam === "local") {
        const cloudOnlyFlags = [
          "type",
          "region",
          "class",
          "default",
          "expiration",
        ] as const;
        for (const flag of cloudOnlyFlags) {
          if (options[flag]) {
            return await ctx.crash({
              exitCode: 1,
              errorType: "fatal",
              printedMessage: `--${flag} cannot be used when creating a local deployment`,
            });
          }
        }
        await createLocalDeployment(
          ctx,
          currentDeployment,
          options.select ?? false,
        );
        return;
      }
    }

    const expiresAt = await resolveExpiresAtOrCrash(ctx, options.expiration);

    const {
      ref,
      regionDetails,
      classDetails,
      projectId,
      type,
      isDefault,
      teamSlug,
      projectSlug,
    } = process.stdin.isTTY
      ? await resolveOptionsInteractively(
          ctx,
          currentDeployment,
          refParam,
          options,
        )
      : await resolveOptionsNoninteractively(
          ctx,
          currentDeployment,
          refParam,
          options,
        );

    showSpinner(
      `Creating ${type} deployment` +
        (regionDetails ? ` in region ${regionDetails.displayName}` : "") +
        (classDetails ? ` with class ${classDetails.type}` : "") +
        "...",
    );

    const created = (
      await typedPlatformClient(ctx).POST(
        "/projects/{project_id}/create_deployment",
        {
          params: {
            path: { project_id: projectId },
          },
          body: {
            type,
            region: regionDetails?.name ?? null,
            reference: ref ?? null,
            isDefault,
            ...(expiresAt !== undefined ? { expiresAt } : {}),
            ...(classDetails ? { class: classDetails.type } : {}),
          },
        },
      )
    ).data!;

    if (created.kind !== "cloud") {
      // This should be impossible
      const err = `Expected cloud deployment to be created but got ${created.kind}`;
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: err,
        errForSentry: err,
      });
    }

    if (!options.select) {
      logFinishedStep(
        `Provisioned a ${created.isDefault ? "default " : ""}${created.deploymentType} deployment.`,
      );
      if (type !== "prod") {
        const selectRef = `${teamSlug}:${projectSlug}:${created.reference}`;
        logMessage(
          `\nTo make \`npx convex\` use this deployment, run ${chalkStderr.bold(`npx convex deployment select ${selectRef}`)}`,
        );
        logMessage(
          chalkStderr.gray(
            "Hint: use `--select` to immediately select the newly created deployment.",
          ),
        );
      }
    }

    if (options.select) {
      const selection: DeploymentSelection = {
        kind: "deploymentWithinProject",
        targetProject: {
          kind: "teamAndProjectSlugs",
          teamSlug,
          projectSlug,
        },
        selectionWithinProject: {
          kind: "deploymentSelector",
          selector: created.reference,
        },
      };
      await saveSelectedDeployment(
        ctx,
        created.reference,
        selection,
        deploymentNameFromSelection(currentDeployment),
      );
    }
  });

async function createLocalDeployment(
  ctx: Context,
  currentDeployment: DeploymentSelection,
  select: boolean,
): Promise<void> {
  const existing = loadProjectLocalConfig(ctx);
  if (existing) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "A local deployment already exists.",
    });
  }

  const { teamSlug, slug: projectSlug } = await resolveProject(
    ctx,
    currentDeployment,
  );

  showSpinner("Downloading local backend...");
  const { version } = await ensureBackendBinaryDownloaded(ctx, {
    kind: "latest",
  });

  const { cloudPort, sitePort } = await chooseLocalBackendPorts(ctx);

  showSpinner("Registering local deployment...");
  const { deploymentName, adminKey } = await bigBrainStart(ctx, {
    port: cloudPort,
    projectSlug,
    teamSlug,
    instanceName: null,
  });

  saveDeploymentConfig(ctx, "local", deploymentName, {
    backendVersion: version,
    ports: { cloud: cloudPort, site: sitePort },
    adminKey,
    instanceSecret: LOCAL_BACKEND_INSTANCE_SECRET,
  });

  logFinishedStep("Created local deployment.");

  if (select) {
    const selection: DeploymentSelection = {
      kind: "deploymentWithinProject",
      targetProject: {
        kind: "deploymentName",
        deploymentName,
        deploymentType: "local",
      },
      selectionWithinProject: {
        kind: "deploymentSelector",
        selector: "local",
      },
    };
    await saveSelectedDeployment(
      ctx,
      "local",
      selection,
      deploymentNameFromSelection(currentDeployment),
    );
  }

  const devCommand = "npx convex dev";
  if (select) {
    logMessage(`\nRun ${chalkStderr.bold(devCommand)} to start it.`);
  } else {
    logMessage(
      `\nTo use this deployment, run:\n` +
        chalkStderr.bold(`      npx convex deployment select local\n`) +
        `  Then, run ${chalkStderr.bold(devCommand)} to start it.`,
    );
  }
}

type RefParam = Parameters<Parameters<typeof deploymentCreate.action>[0]>[0];
type OptionsParam = Parameters<
  Parameters<typeof deploymentCreate.action>[0]
>[1];

async function resolveOptionsNoninteractively(
  ctx: Context,
  currentDeployment: DeploymentSelection,
  refParam: RefParam,
  options: OptionsParam,
) {
  let ref: string | undefined;
  let teamAndProject: { teamSlug: string; projectSlug: string } | undefined;
  if (refParam) {
    const result = parseSelectorForNewDeployment(refParam);
    if (result.kind === "invalid") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: result.message,
      });
    }
    ref = result.ref;
    teamAndProject = result.teamAndProject;
  }

  if (!ref && !options.default) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "Specify a deployment ref or use --default:\n" +
        "  `npx convex deployment create my-deployment-ref --type dev`\n" +
        "  `npx convex deployment create --type prod --default`",
    });
  }

  if (!options.type) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `--type is required (supported values: ${SUPPORTED_TYPES.join(", ")})`,
    });
  }

  const project = teamAndProject
    ? await getProjectDetails(ctx, {
        kind: "teamAndProjectSlugs",
        teamSlug: teamAndProject.teamSlug,
        projectSlug: teamAndProject.projectSlug,
      })
    : await resolveProject(ctx, currentDeployment);
  const projectId = project.id;

  // If no region is passed in, the team's default region will be used
  let regionDetails: AvailableRegion | null = null;
  if (options.region) {
    const availableRegions = await fetchAvailableRegions(ctx, project.teamId);
    regionDetails = await resolveRegionDetailsOrCrash(
      ctx,
      availableRegions,
      options.region,
    );
  }

  // If no class is passed in, the team's default class will be used
  let classDetails: AvailableClass | null = null;
  if (options.class) {
    const availableClasses = await fetchAvailableClasses(ctx, project.teamId);
    classDetails = await resolveClassDetailsOrCrash(
      ctx,
      availableClasses,
      options.class,
    );
  }

  return {
    ref,
    isDefault: options.default ?? null,
    projectId,
    regionDetails,
    classDetails,
    type: options.type,
    teamSlug: project.teamSlug,
    projectSlug: project.slug,
  };
}

async function resolveOptionsInteractively(
  ctx: Context,
  currentDeployment: DeploymentSelection,
  refParam: RefParam,
  options: OptionsParam,
) {
  let deploymentType: "dev" | "prod" | "preview";
  if (options.type) {
    deploymentType = logAndUse("type", options.type);
  } else {
    const dtypeChoices = [
      {
        name: "dev",
        value: "dev" as const,
      },
      {
        name: "preview",
        value: "preview" as const,
      },
      {
        name: "prod",
        value: "prod" as const,
      },
    ];
    deploymentType = await promptOptions(ctx, {
      message: "Deployment type?",
      choices: dtypeChoices,
    });
  }

  let ref: string | undefined;
  let teamAndProject: { teamSlug: string; projectSlug: string } | undefined;
  if (refParam) {
    const result = parseSelectorForNewDeployment(refParam);
    if (result.kind === "invalid") {
      logFailure(result.message);
    } else {
      ref = logAndUse("ref", result.ref);
      teamAndProject = result.teamAndProject;
    }
  }
  while (ref === undefined) {
    const gitDefault = defaultRef(localGitBranch(), deploymentType);
    const input = await promptString(ctx, {
      message:
        "What do you want to call this deployment?\n" +
        chalkStderr.reset.dim(
          "The deployment reference will be used to identify your deployment on the dashboard and in CLI commands.\nExamples: staging, dev/james/feature",
        ) +
        "\n>",
      ...(gitDefault !== undefined ? { default: gitDefault } : {}),
      validate: validateTentativeReference,
    });
    const result = parseSelectorForNewDeployment(input);
    if (result.kind === "invalid") {
      logFailure(result.message);
      continue;
    }
    ref = result.ref;
    teamAndProject = result.teamAndProject;
  }

  const project = teamAndProject
    ? await getProjectDetails(ctx, {
        kind: "teamAndProjectSlugs",
        teamSlug: teamAndProject.teamSlug,
        projectSlug: teamAndProject.projectSlug,
      })
    : await resolveProject(ctx, currentDeployment);

  const availableRegions = await fetchAvailableRegions(ctx, project.teamId);
  let regionDetails: AvailableRegion;
  if (options.region) {
    regionDetails = await resolveRegionDetailsOrCrash(
      ctx,
      availableRegions,
      options.region,
    );
    logAndUse("region", regionDetails.displayName);
  } else {
    // Use the team's default region if set, or prompt the user to pick
    // TODO: this duplicates some of the logic in selectRegionOrUseDefault (npm-packages/convex/src/cli/lib/utils/utils.ts)
    const teams = (await typedBigBrainClient(ctx).GET("/teams")).data!;
    const team = teams.find((team) => team.slug === project.teamSlug);
    if (!team) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Error: Team ${project.teamSlug} not found.`,
      });
    }
    const regionName =
      team.defaultRegion ?? (await selectRegion(ctx, team.id, deploymentType));
    regionDetails = await resolveRegionDetailsOrCrash(
      ctx,
      availableRegions,
      regionName,
    );
    if (team.defaultRegion) {
      logFinishedStep(
        `Using team default region of ${regionDetails.displayName}`,
      );
    } else {
      await logNoDefaultRegionMessage(team.slug);
    }
  }

  let classDetails: AvailableClass | null = null;
  if (options.class) {
    const availableClasses = await fetchAvailableClasses(ctx, project.teamId);
    classDetails = await resolveClassDetailsOrCrash(
      ctx,
      availableClasses,
      options.class,
    );
    logAndUse("class", classDetails.type);
  }

  return {
    ref,
    isDefault: options.default ?? null,
    projectId: project.id,
    regionDetails,
    classDetails,
    type: deploymentType,
    teamSlug: project.teamSlug,
    projectSlug: project.slug,
  };
}

type NewDeploymentSelectorResult =
  | {
      kind: "valid";
      ref: string;
      teamAndProject?: { teamSlug: string; projectSlug: string };
    }
  | { kind: "invalid"; message: string };

function parseSelectorForNewDeployment(
  selectorString: string,
): NewDeploymentSelectorResult {
  const selector = parseDeploymentSelector(selectorString);
  switch (selector.kind) {
    case "local":
      return {
        kind: "invalid",
        message: `"local" is reserved as an alias for your local deployment. To create one, run ${chalkStderr.bold("npx convex deployment create local")}`,
      };
    case "deploymentName":
      return {
        kind: "invalid",
        message: `"${selector.deploymentName}" is not a valid deployment reference. References can't look like "word-word-123" — that format is reserved for automatically-generated deployment names.`,
      };
    case "inCurrentProject": {
      const inner = selector.selector;
      if (inner.kind === "dev") {
        return {
          kind: "invalid",
          message: `"dev" is reserved as an alias for your default dev deployment.`,
        };
      }
      if (inner.kind === "prod") {
        return {
          kind: "invalid",
          message: `"prod" is reserved as an alias for your default production deployment.`,
        };
      }
      return { kind: "valid", ref: inner.reference };
    }
    case "inProject": {
      return {
        kind: "invalid",
        message: `Please use "team:project:ref" to specify the team when creating a new deployment in a different project.`,
      };
    }
    case "inTeamProject": {
      const inner = selector.selector;
      if (inner.kind === "dev") {
        return {
          kind: "invalid",
          message: `"dev" is reserved as an alias for your default dev deployment.`,
        };
      }
      if (inner.kind === "prod") {
        return {
          kind: "invalid",
          message: `"prod" is reserved as an alias for your default production deployment.`,
        };
      }
      return {
        kind: "valid",
        ref: inner.reference,
        teamAndProject: {
          teamSlug: selector.teamSlug,
          projectSlug: selector.projectSlug,
        },
      };
    }
    default:
      selector satisfies never;
      return {
        kind: "invalid",
        message: "Unknown state. This is a bug in Convex.",
      };
  }
}

async function resolveProject(
  ctx: Context,
  deploymentSelection: DeploymentSelection,
): Promise<PlatformProjectDetails> {
  switch (deploymentSelection.kind) {
    case "existingDeployment": {
      const { deploymentFields } = deploymentSelection.deploymentToActOn;
      if (deploymentFields) {
        return await getProjectDetails(ctx, {
          kind: "deploymentName",
          deploymentName: deploymentFields.deploymentName,
          deploymentType: null,
        });
      }
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          "Cannot infer project from the current deployment configuration. Use `team:project:ref` to specify team and project slugs.",
      });
    }
    case "deploymentWithinProject": {
      return await getProjectDetails(ctx, deploymentSelection.targetProject);
    }
    case "preview": {
      const slugs = await getTeamAndProjectFromPreviewAdminKey(
        ctx,
        deploymentSelection.previewDeployKey,
      );
      return await getProjectDetails(ctx, {
        kind: "teamAndProjectSlugs",
        teamSlug: slugs.teamSlug,
        projectSlug: slugs.projectSlug,
      });
    }
    case "chooseProject":
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          "No project configured yet. Use `team:project:ref` to specify team and project slugs.",
      });
    case "anonymous":
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          "Cannot create a deployment in anonymous mode. " +
          "Run `npx convex login` and configure a project first.",
      });
    default: {
      deploymentSelection satisfies never;
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Unexpected deployment selection kind.`,
      });
    }
  }
}

const REGION_NAME_TO_ALIAS: Record<string, string> = {
  "aws-us-east-1": "us",
  "aws-eu-west-1": "eu",
};

const REGION_ALIAS_TO_NAME = Object.fromEntries(
  Object.entries(REGION_NAME_TO_ALIAS).map(([name, alias]) => [alias, name]),
);

export async function fetchAvailableRegions(ctx: Context, teamId: number) {
  const regionsResponse = (
    await typedPlatformClient(ctx).GET(
      "/teams/{team_id}/list_deployment_regions",
      {
        params: {
          path: { team_id: `${teamId}` },
        },
      },
    )
  ).data!;
  return regionsResponse.items.filter((item) => item.available);
}

type AvailableRegion = Awaited<
  ReturnType<typeof fetchAvailableRegions>
>[number];

export function resolveRegionDetails(
  availableRegions: AvailableRegion[],
  region: string,
) {
  const resolvedRegion = REGION_ALIAS_TO_NAME[region] ?? region;
  return availableRegions.find((item) => item.name === resolvedRegion) ?? null;
}

async function resolveRegionDetailsOrCrash(
  ctx: Context,
  availableRegions: AvailableRegion[],
  region: string,
) {
  const regionDetails = resolveRegionDetails(availableRegions, region);
  if (!regionDetails) {
    return await crashInvalidRegion(ctx, availableRegions, region);
  }
  return regionDetails;
}

function invalidRegionMessage(
  availableRegions: AvailableRegion[],
  region: string,
): string {
  const formatted = availableRegions
    .map(
      (item) =>
        `    Use \`--region ${REGION_NAME_TO_ALIAS[item.name] ?? item.name}\` for ${item.displayName}`,
    )
    .join("\n");
  return `Invalid region "${region}".\n\n` + formatted;
}

async function crashInvalidRegion(
  ctx: Context,
  availableRegions: AvailableRegion[],
  region: string,
): Promise<never> {
  return await ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage: invalidRegionMessage(availableRegions, region),
  });
}

export async function fetchAvailableClasses(ctx: Context, teamId: number) {
  const classesResponse = (
    await typedPlatformClient(ctx).GET(
      "/teams/{team_id}/list_deployment_classes",
      {
        params: {
          path: { team_id: `${teamId}` },
        },
      },
    )
  ).data!;
  return classesResponse.items.filter((item) => item.available);
}

type AvailableClass = Awaited<ReturnType<typeof fetchAvailableClasses>>[number];

export function resolveClassDetails(
  availableClasses: AvailableClass[],
  className: string,
) {
  return availableClasses.find((item) => item.type === className) ?? null;
}

async function resolveClassDetailsOrCrash(
  ctx: Context,
  availableClasses: AvailableClass[],
  className: string,
) {
  const classDetails = resolveClassDetails(availableClasses, className);
  if (!classDetails) {
    return await crashInvalidClass(ctx, availableClasses, className);
  }
  return classDetails;
}

function invalidClassMessage(
  availableClasses: AvailableClass[],
  className: string,
): string {
  const formatted = availableClasses
    .map((item) => `    \`--class ${item.type}\``)
    .join("\n");
  return `Invalid class "${className}".\n\nAvailable classes:\n` + formatted;
}

async function crashInvalidClass(
  ctx: Context,
  availableClasses: AvailableClass[],
  className: string,
): Promise<never> {
  return await ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage: invalidClassMessage(availableClasses, className),
  });
}

async function resolveExpiresAtOrCrash(
  ctx: Context,
  expiration: string | undefined,
): Promise<number | null | undefined> {
  if (!expiration) {
    return undefined;
  }
  const parsed = parseExpiration(expiration);
  if (parsed.kind === "error") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: parsed.message,
    });
  }
  const now = Date.now();
  const resolved = resolveExpiration(parsed, now);
  if (resolved !== null) {
    const validation = validateExpiration(resolved, now);
    if (validation.kind === "error") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: validation.message,
      });
    }
  }
  return resolved;
}

/**
 * Helper to log a value passed in as a CLI argument in the interactive flow.
 */
function logAndUse<T extends string | boolean>(label: string, value: T): T {
  logFinishedStep(`Using ${label}: ${chalkStderr.bold(value)}`);
  return value;
}

// This is an oversimplification, it’s fine if it fails later
function validateTentativeReference(tentativeReference: string): true | string {
  if (tentativeReference.length < 3) {
    return "References must be at least 3 characters";
  }
  if (tentativeReference.length > 100) {
    return "References must be at most 100 characters";
  }
  if (!/^[a-z0-9/-]+$/.test(tentativeReference)) {
    return "References can only contain lowercase letters, numbers, `-`, and `/`";
  }
  if (tentativeReference === "dev") {
    return '"dev" is reserved as an alias for your default dev deployment.';
  }
  if (tentativeReference === "prod") {
    return '"prod" is reserved as an alias for your default production deployment.';
  }
  if (tentativeReference === "local") {
    return `"local" is reserved as an alias for your local deployment. To create one, run ${chalkStderr.bold("npx convex deployment create local")}`;
  }
  if (/^[a-z]+-[a-z]+-\d+$/.test(tentativeReference)) {
    return 'References can\'t look like "word-word-123" — that format is reserved for automatically-generated deployment names. Try something like dev/my-feature or staging instead.';
  }

  return true;
}

/**
 * Get the current local git branch name by shelling out to git.
 * Returns null if git is unavailable, the repo is in detached HEAD state,
 * or the branch is main/master.
 */
function localGitBranch(): string | null {
  try {
    const branch = (
      execSync("git rev-parse --abbrev-ref HEAD", {
        stdio: ["pipe", "pipe", "pipe"],
        timeout: 5000,
      }) as Buffer
    )
      .toString()
      .trim();
    if (
      !branch ||
      branch === "HEAD" ||
      branch === "main" ||
      branch === "master"
    ) {
      return null;
    }
    return branch;
  } catch {
    return null;
  }
}

/**
 * Slugify a git branch name into a valid deployment reference.
 * Returns undefined if the result would fail validation.
 */
function defaultRef(
  branch: string | null,
  deploymentType: "dev" | "prod" | "preview",
): string | undefined {
  if (deploymentType !== "dev" && deploymentType !== "preview") {
    return undefined;
  }
  if (!branch) return undefined;
  const slug = branch
    .replace(/[^a-z0-9/-]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  if (!slug) return undefined;
  const ref = `${deploymentType}/${slug}`;
  const valid = validateTentativeReference(ref);
  if (valid !== true) return undefined;
  return ref;
}
