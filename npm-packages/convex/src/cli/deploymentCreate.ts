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
} from "./lib/deploymentSelection.js";
import {
  logNoDefaultRegionMessage,
  selectRegion,
  typedBigBrainClient,
  typedPlatformClient,
} from "./lib/utils/utils.js";
import { PlatformProjectDetails } from "@convex-dev/platform/managementApi";
import { getTeamAndProjectFromPreviewAdminKey } from "./lib/deployment.js";
import { selectDeployment } from "./deploymentSelect.js";
import { promptOptions, promptString } from "./lib/utils/prompts.js";
import { chalkStderr } from "chalk";
import { parseDeploymentSelector } from "./lib/deploymentSelector.js";

export const deploymentCreate = new Command("create")
  .summary("Create a new cloud deployment for a project")
  .description(
    "Create a new cloud deployment for a project.\n\n" +
      "  Create a dev deployment and select it: `npx convex deployment create dev/my-new-feature --type dev --select`\n" +
      "  Create a prod staging deployment:      `npx convex deployment create staging --type prod`\n",
  )
  .argument("[ref]")
  .allowExcessArguments(false)
  .addOption(
    new Option("--type <type>", "Deployment type").choices([
      "dev",
      "prod",
      "preview",
    ] as const),
  )
  .option("--region <region>", "Deployment region")
  .option(
    "--select",
    "Select the new deployment. This will update the Convex environment variables in .env.local and `npx convex dev` will run against this deployment.",
  )
  .option(
    "--default",
    "Set the new deployment as your default development or production deployment.",
  )
  .action(async (refParam, options) => {
    const ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });

    const { ref, regionDetails, projectId, type, isDefault } = process.stdin
      .isTTY
      ? await resolveOptionsInteractively(ctx, refParam, options)
      : await resolveOptionsNoninteractively(ctx, refParam, options);

    showSpinner(
      `Creating ${type} deployment` +
        (regionDetails ? ` in region ${regionDetails.displayName}` : "") +
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
        `Provisioned a ${created.isDefault ? "default " : ""}${created.deploymentType} deployment. Select this deployment to develop against using \`npx convex deployment select ${created.reference}\``,
      );
      logMessage(
        chalkStderr.gray(
          "Hint: use `npx convex deployment create --select` to immediately select the newly created deployment.",
        ),
      );
    }

    if (options.select) {
      await selectDeployment(ctx, created.reference);
    }
  });

type RefParam = Parameters<Parameters<typeof deploymentCreate.action>[0]>[0];
type OptionsParam = Parameters<
  Parameters<typeof deploymentCreate.action>[0]
>[1];

async function resolveOptionsNoninteractively(
  ctx: Context,
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
      printedMessage: "--type is required. Use --type dev or --type prod.",
    });
  }

  const project = teamAndProject
    ? await getProjectDetails(ctx, {
        kind: "teamAndProjectSlugs",
        teamSlug: teamAndProject.teamSlug,
        projectSlug: teamAndProject.projectSlug,
      })
    : await resolveProject(
        ctx,
        await getDeploymentSelection(ctx, {
          url: undefined,
          adminKey: undefined,
          envFile: undefined,
        }),
      );
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

  return {
    ref,
    isDefault: options.default ?? null,
    projectId,
    regionDetails,
    type: options.type,
  };
}

async function resolveOptionsInteractively(
  ctx: Context,
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
    const input = await promptString(ctx, { message: "Deployment ref?" });
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
    : await resolveProject(
        ctx,
        await getDeploymentSelection(ctx, {
          url: undefined,
          adminKey: undefined,
          envFile: undefined,
        }),
      );

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
      logNoDefaultRegionMessage(team.slug);
    }
  }

  return {
    ref,
    isDefault: options.default ?? null,
    projectId: project.id,
    regionDetails,
    type: deploymentType,
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
    case "deploymentName":
      return {
        kind: "invalid",
        message: `"${selector.deploymentName}" is not a valid deployment reference. References cannot be in the format abc-xyz-123, as it is reserved for deployment names.`,
      };
    case "inCurrentProject": {
      const inner = selector.selector;
      if (inner.kind === "dev") {
        return {
          kind: "invalid",
          message: `"dev" is not a valid deployment reference.`,
        };
      }
      if (inner.kind === "prod") {
        return {
          kind: "invalid",
          message: `"prod" is not a valid deployment reference.`,
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
          message: `"dev" is not a valid deployment reference.`,
        };
      }
      if (inner.kind === "prod") {
        return {
          kind: "invalid",
          message: `"prod" is not a valid deployment reference.`,
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

/**
 * Helper to log a value passed in as a CLI argument in the interactive flow.
 */
function logAndUse<T extends string | boolean>(label: string, value: T): T {
  logFinishedStep(`Using ${label}: ${chalkStderr.bold(value)}`);
  return value;
}
