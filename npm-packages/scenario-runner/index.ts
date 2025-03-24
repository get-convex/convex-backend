import { Command } from "@commander-js/extra-typings";
import { WebSocket } from "ws";
import { RunFunction } from "./scenarios/run_function.js";
import { ObserveInsert } from "./scenarios/observe_insert.js";
import "@sentry/tracing";
import * as Sentry from "@sentry/node";
import { SnapshotExport } from "./scenarios/snapshot_export.js";
import { CloudBackup } from "./scenarios/cloud_backup.js";
import { Config, ProvisionerInfo, Scenario } from "./scenario.js";
import { Search } from "./scenarios/search.js";
import { VectorSearch } from "./scenarios/vector_search.js";
import { ConvexClient } from "convex/browser";
import { ScenarioMessage } from "./types.js";
import { RunHttpAction } from "./scenarios/run_http_action.js";
import dns from "node:dns";

Sentry.init({
  tracesSampleRate: 0.1,
});

/**
 * Node defaults to ipv6, and since usher runs locally with ipv4 addresses,
 * set the default result order to ipv4
 */
dns.setDefaultResultOrder("ipv4first");

async function main(
  deploymentUrl: string,
  adminKey: string,
  lgPort: number,
  provisionHost: string | undefined,
  accessToken: string | undefined,
  scenarios: ScenarioMessage[],
) {
  console.log(`ScenarioRunner is running! ${deploymentUrl}.`);
  const ws = new WebSocket(`ws://127.0.0.1:${lgPort}/sync`);

  const provisionerInfo =
    provisionHost && accessToken
      ? await getProvisionerInfo(provisionHost, accessToken, deploymentUrl)
      : undefined;

  const config = {
    deploymentUrl,
    loadGenWS: ws,
    provisionerInfo,
  };
  await Promise.all(
    scenarios.map((scenarioMessage) =>
      runScenario(config, scenarioMessage, adminKey),
    ),
  );
}

async function getProvisionerInfo(
  provisionHost: string,
  accessToken: string,
  deploymentUrl: string,
): Promise<ProvisionerInfo> {
  const deploymentName = await (
    await fetch(`${deploymentUrl}/instance_name`)
  ).text();

  const response = await fetch(
    `${provisionHost}/api/deployment/${deploymentName}/team_and_project`,
    {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    const responseText = await response.text();
    throw new Error(
      `HTTP error ${response.status}: ${response.statusText}. Response: ${responseText}`,
    );
  }

  let info;
  try {
    info = await response.json();
  } catch (e: unknown) {
    const responseText = await response.text();
    throw new Error(
      `Failed to parse JSON response: ${e instanceof Error ? e.message : String(e)}. Full response: ${responseText}`,
    );
  }
  const deploymentId = info.deploymentId;

  return {
    provisionHost,
    accessToken,
    deploymentName,
    deploymentId,
  };
}

async function runScenario(
  config: Config,
  scenarioMessage: ScenarioMessage,
  adminKey: string,
) {
  let scenario: Scenario | undefined;
  const scenarioSpec = scenarioMessage.scenario;
  console.log(`Running scenario: ${scenarioSpec.name}`);
  switch (scenarioSpec.name) {
    case "RunFunction":
      scenario = new RunFunction(
        config,
        scenarioSpec.path,
        scenarioSpec.fn_type,
      );
      break;
    case "ObserveInsert":
      scenario = new ObserveInsert(config, scenarioSpec.search_indexes);
      break;
    case "SnapshotExport":
      scenario = new SnapshotExport(config, adminKey);
      break;
    case "CloudBackup":
      scenario = new CloudBackup(config);
      break;
    case "Search":
      scenario = new Search(config);
      break;
    case "VectorSearch":
      scenario = new VectorSearch(config);
      break;
    case "RunHttpAction":
      scenario = new RunHttpAction(
        config,
        scenarioSpec.path,
        scenarioSpec.method,
      );
      break;
    default: {
      const _typeCheck: never = scenarioSpec;
      throw new Error(`Invalid scenario: ${scenarioMessage}`);
    }
  }
  if (scenario) {
    const client = new ConvexClient(config.deploymentUrl);
    const runScenario = async () => {
      const client = new ConvexClient(config.deploymentUrl);
      await scenario!.run(client);
      await client.close();
    };
    const rate = scenarioMessage.rate;
    const handleError = (err: Error) => {
      scenario!.sendDefaultError(err);
      Sentry.captureException(err);
      console.error(
        `Failed to run scenario ${scenario!.name} with error: ${err}`,
      );
    };
    try {
      if (rate === null) {
        // Benchmark mode
        const numThreads = scenarioMessage.threads || 1; // Default to 1 thread if not specified

        // Create the specified number of threads
        const threads = Array.from({ length: numThreads }, () => {
          const client = new ConvexClient(config.deploymentUrl);
          return (async () => {
            // Each thread runs scenarios in a loop
            for (;;) {
              await scenario!.run(client).catch((err) => handleError(err));
              scenario!.cleanUp();
            }
          })();
        });

        // Wait for all threads (they'll run until the process is terminated)
        await Promise.all(threads);
      } else {
        if (rate === 0) {
          return;
        }
        for (;;) {
          const averageDelay = 1000 / rate;
          const delayMax = 2 * averageDelay;
          const actualDelay = Math.random() * delayMax;
          await Promise.all([
            new Promise((resolve) => setTimeout(resolve, actualDelay)),
            runScenario(),
          ]).catch((err) => handleError(err));
          scenario.cleanUp();
        }
      }
    } catch (err) {
      Sentry.captureException(err);
      console.error(
        `Failed to run scenario ${scenario} in a loop with error: ${err}`,
      );
    } finally {
      scenario.cleanUp();
      await client.close();
    }
  } else {
    console.error(
      `Received invalid message: ${scenarioSpec.name}. Messages must be a Scenario.`,
    );
  }
}

const program = new Command();
program
  .name("scenario-runner")
  .description(
    "scenario-runner runs client-side scenarios against test deployments",
  )
  .usage("command url [options]")
  .requiredOption(
    "--deployment-url <url>",
    "URL of the deployment to run scenarios against",
  )
  .requiredOption("--admin-key <admin_key>", "Admin key to access deployment")
  .requiredOption(
    "--scenarios <scenarios>",
    "JSON scenarios to run against the given deployment",
  )
  .requiredOption(
    "--load-generator-port <port>",
    "Port to connect to load generator",
  )
  .option("--provision-host <host>", "Port to connect to big brain")
  .option("--access-token <token>", "Access token for talking to big-bran")
  .action(async (options) => {
    const scenarios = JSON.parse(options.scenarios);
    await main(
      options.deploymentUrl,
      options.adminKey,
      Number(options.loadGeneratorPort),
      options.provisionHost,
      options.accessToken,
      scenarios.scenarios,
    );
  });
void program.parseAsync(process.argv);
