import { Context } from "../../../bundler/context.js";
import { logMessage } from "../../../bundler/log.js";
import { detect } from "detect-port";
import crypto from "crypto";
import { chalkStderr } from "chalk";
import { suggestedEnvVarName } from "../envvars.js";
import * as dotenv from "dotenv";
import { ENV_VAR_FILE_PATH } from "../utils/utils.js";

function extractPortFromUrl(url: string | undefined): number | null {
  if (!url) {
    return null;
  }
  try {
    const parsedUrl = new URL(url);
    if (parsedUrl.port) {
      const port = parseInt(parsedUrl.port, 10);
      if (port >= 1024 && port <= 65535) {
        return port;
      }
    }
  } catch {
    return null;
  }
  return null;
}

/**
 * Read port settings from .env.local based on detected framework.
 */
export async function getPortsFromEnvFile(
  ctx: Context,
  envFile?: string,
): Promise<{ cloud: number | null; site: number | null }> {
  const envFilePath = envFile || ENV_VAR_FILE_PATH;
  
  if (!ctx.fs.exists(envFilePath)) {
    return { cloud: null, site: null };
  }

  const existingFile = ctx.fs.readUtf8File(envFilePath);
  const config = dotenv.parse(existingFile);

  const { envVar, publicPrefix } = await suggestedEnvVarName(ctx);
  
  const baseEnvVarName = envVar;
  const siteEnvVarName = publicPrefix 
    ? `${publicPrefix}CONVEX_SITE_URL`
    : "CONVEX_SITE_URL";

  const cloudPort = extractPortFromUrl(config[baseEnvVarName]);
  const sitePort = extractPortFromUrl(config[siteEnvVarName]);

  return { cloud: cloudPort, site: sitePort };
}

export async function choosePorts(
  ctx: Context,
  {
    count,
    requestedPorts,
    startPort,
  }: {
    count: number;
    requestedPorts?: Array<number | null>;
    startPort: number;
  },
): Promise<Array<number>> {
  const ports: Array<number> = [];
  for (let i = 0; i < count; i++) {
    const requestedPort = requestedPorts?.[i];
    if (requestedPort !== null) {
      const port = await detect(requestedPort);
      if (port !== requestedPort) {
        return ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: "Requested port is not available",
        });
      }
      ports.push(port);
    } else {
      const portToTry =
        ports.length > 0 ? ports[ports.length - 1] + 1 : startPort;
      const port = await detect(portToTry);
      ports.push(port);
    }
  }
  return ports;
}

export async function isOffline(): Promise<boolean> {
  // TODO(ENG-7080) -- implement this for real
  return false;
}

export function printLocalDeploymentWelcomeMessage() {
  logMessage(
    chalkStderr.cyan("You're trying out the beta local deployment feature!"),
  );
  logMessage(
    chalkStderr.cyan(
      "To learn more, read the docs: https://docs.convex.dev/cli/local-deployments",
    ),
  );
  logMessage(
    chalkStderr.cyan(
      "To opt out at any time, run `npx convex disable-local-deployments`",
    ),
  );
}

export function generateInstanceSecret(): string {
  return crypto.randomBytes(32).toString("hex");
}

export const LOCAL_BACKEND_INSTANCE_SECRET =
  "4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974";
