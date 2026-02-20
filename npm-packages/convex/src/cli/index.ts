import { Command } from "@commander-js/extra-typings";
import { init } from "./init.js";
import { dashboard } from "./dashboard.js";
import { deployments } from "./deployments.js";
import { docs } from "./docs.js";
import { run } from "./run.js";
import { version } from "./version.js";
import { auth } from "./auth.js";
import { codegen } from "./codegen.js";
import { reinit } from "./reinit.js";
import { update } from "./update.js";
import { typecheck } from "./typecheck.js";
import { login } from "./login.js";
import { logout } from "./logout.js";
import { chalkStderr } from "chalk";
import * as Sentry from "@sentry/node";
import { initSentry } from "./lib/utils/sentry.js";
import { dev } from "./dev.js";
import { deploy } from "./deploy.js";
import { logs } from "./logs.js";
import { networkTest } from "./network_test.js";
import { convexExport } from "./convexExport.js";
import { convexImport } from "./convexImport.js";
import { env } from "./env.js";
import { data } from "./data.js";
import { format } from "util";
import { functionSpec } from "./functionSpec.js";
import { insights } from "./insights.js";
import { disableLocalDeployments } from "./disableLocalDev.js";
import { mcp } from "./mcp.js";
import dns from "node:dns";
import net from "node:net";
import { integration } from "./integration.js";
import { setGlobalDispatcher, EnvHttpProxyAgent } from "undici";
import { logVerbose } from "../bundler/log.js";

const HARD_MINIMUM_NODE_MAJOR_VERSION = 16;
const HARD_MINIMUM_NODE_MINOR_VERSION = 15;
const SOFT_MINIMUM_NODE_MAJOR_VERSION = 20;

// console.error before it started being red by default in Node.js v20
function logToStderr(...args: unknown[]) {
  process.stderr.write(`${format(...args)}\n`);
}

async function main() {
  const nodeVersion = process.versions.node;
  const majorVersion = parseInt(nodeVersion.split(".")[0], 10);
  const minorVersion = parseInt(nodeVersion.split(".")[1], 10);

  const proxy = process.env.HTTPS_PROXY || process.env.HTTP_PROXY;
  if (proxy) {
    setGlobalDispatcher(new EnvHttpProxyAgent());
    logVerbose(`[proxy-bootstrap] Using proxy: ${proxy}`);
  }

  // Use ipv4 first for 127.0.0.1 in tests
  dns.setDefaultResultOrder("ipv4first");

  // Increase the timeout from default 250ms for high latency situations,
  // see https://github.com/nodejs/node/issues/54359.
  if (majorVersion >= 20) {
    // While we use Node.js v18 types
    (net as any).setDefaultAutoSelectFamilyAttemptTimeout?.(1000);
  }

  initSentry();

  if (
    majorVersion < HARD_MINIMUM_NODE_MAJOR_VERSION ||
    (majorVersion === HARD_MINIMUM_NODE_MAJOR_VERSION &&
      minorVersion < HARD_MINIMUM_NODE_MINOR_VERSION)
  ) {
    logToStderr(
      chalkStderr.red(
        `Your Node version ${nodeVersion} is too old. Convex requires at least Node v${HARD_MINIMUM_NODE_MAJOR_VERSION}.${HARD_MINIMUM_NODE_MINOR_VERSION}`,
      ),
    );
    logToStderr(
      chalkStderr.gray(
        `You can use ${chalkStderr.bold(
          "nvm",
        )} (https://github.com/nvm-sh/nvm#installing-and-updating) to manage different versions of Node.`,
      ),
    );
    logToStderr(
      chalkStderr.gray(
        "After installing `nvm`, install the latest version of Node with " +
          chalkStderr.bold("`nvm install node`."),
      ),
    );
    logToStderr(
      chalkStderr.gray(
        "Then, activate the installed version in your terminal with " +
          chalkStderr.bold("`nvm use`."),
      ),
    );
    process.exit(1);
  }

  if (majorVersion < SOFT_MINIMUM_NODE_MAJOR_VERSION) {
    logToStderr(
      chalkStderr.yellow(
        `Warning: Your Node version ${nodeVersion} is below the recommended minimum of Node v${SOFT_MINIMUM_NODE_MAJOR_VERSION}.x. Convex may work but could behave unexpectedly.`,
      ),
    );
    logToStderr(
      chalkStderr.gray(
        `We recommend upgrading Node to v${SOFT_MINIMUM_NODE_MAJOR_VERSION} or newer.`,
      ),
    );
    logToStderr(
      chalkStderr.gray(
        `You can use ${chalkStderr.bold(
          "nvm",
        )} (https://github.com/nvm-sh/nvm#installing-and-updating) to manage different versions of Node.`,
      ),
    );
    logToStderr(
      chalkStderr.gray(
        "After installing `nvm`, install the latest version of Node with " +
          chalkStderr.bold("`nvm install node`."),
      ),
    );
    logToStderr(
      chalkStderr.gray(
        "Then, activate the installed version in your terminal with " +
          chalkStderr.bold("`nvm use`."),
      ),
    );
  }

  const program = new Command();
  program
    .name("convex")
    .usage("<command> [options]")
    .description("Start developing with Convex by running `npx convex dev`.")
    .addCommand(login, { hidden: true })
    .addCommand(init, { hidden: true })
    .addCommand(reinit, { hidden: true })
    .addCommand(dev)
    .addCommand(deploy)
    .addCommand(deployments, { hidden: true })
    .addCommand(run)
    .addCommand(convexImport)
    .addCommand(dashboard)
    .addCommand(docs)
    .addCommand(logs)
    .addCommand(typecheck, { hidden: true })
    .addCommand(auth, { hidden: true })
    .addCommand(convexExport)
    .addCommand(env)
    .addCommand(data)
    .addCommand(codegen)
    .addCommand(update)
    .addCommand(logout)
    .addCommand(networkTest, { hidden: true })
    .addCommand(integration, { hidden: true })
    .addCommand(functionSpec)
    .addCommand(insights)
    .addCommand(disableLocalDeployments)
    .addCommand(mcp)
    .addHelpCommand("help <command>", "Show help for given <command>")
    .version(version)
    // Hide version and help so they don't clutter
    // the list of commands.
    .configureHelp({ visibleOptions: () => [] })
    .showHelpAfterError();

  // Run the command and be sure to flush Sentry before exiting.
  try {
    await program.parseAsync(process.argv);
  } catch (e) {
    Sentry.captureException(e);
    process.exitCode = 1;
    // This is too early to use `logError`, so just log directly.
    // eslint-disable-next-line no-console
    console.error(chalkStderr.red("Unexpected Error: " + e));
  } finally {
    await Sentry.close();
  }
  process.exit();
}
void main();
