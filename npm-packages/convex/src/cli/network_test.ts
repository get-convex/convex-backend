import { Command, Option } from "@commander-js/extra-typings";
import {
  DeploymentSelection,
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import {
  Context,
  logFailure,
  logFinishedStep,
  logMessage,
  oneoffContext,
  showSpinner,
} from "../bundler/context.js";
import * as net from "net";
import * as dns from "dns";
import * as crypto from "crypto";
import { deploymentClient, formatDuration, formatSize } from "./lib/utils.js";
import chalk from "chalk";

const ipFamilyNumbers = { ipv4: 4, ipv6: 6, auto: 0 } as const;
const ipFamilyNames = { 4: "ipv4", 6: "ipv6", 0: "auto" } as const;

export const networkTest = new Command("network-test")
  .description("Run a network test to Convex's servers")
  .addOption(
    new Option(
      "--timeout <timeout>",
      "Timeout in seconds for the network test (default: 10).",
    ),
  )
  .addOption(
    new Option(
      "--ip-family <ipFamily>",
      "IP family to use (ipv4, ipv6, or auto)",
    ),
  )
  .addOption(
    new Option(
      "--prod",
      "Perform the network test on this project's production deployment. Defaults to your dev deployment without this flag.",
    ).conflicts(["--preview-name", "--deployment-name", "--url"]),
  )
  .addOption(
    new Option(
      "--preview-name <previewName>",
      "Perform the network test on the preview deployment with the given name. Defaults to your dev deployment without this flag.",
    ).conflicts(["--prod", "--deployment-name", "--url"]),
  )
  .addOption(
    new Option(
      "--deployment-name <deploymentName>",
      "Perform the network test on the specified deployment. Defaults to your dev deployment without this flag.",
    ).conflicts(["--prod", "--preview-name", "--url"]),
  )
  .addOption(
    new Option("--url <url>")
      .conflicts(["--prod", "--preview-name", "--deployment-name"])
      .hideHelp(),
  )
  .addOption(new Option("--admin-key <adminKey>").hideHelp())

  .addOption(new Option("--url <url>"))
  .action(async (options) => {
    const ctx = oneoffContext;
    const timeoutSeconds = options.timeout
      ? Number.parseFloat(options.timeout)
      : 10;
    await withTimeout(
      ctx,
      "Network test",
      timeoutSeconds * 1000,
      runNetworkTest(ctx, options),
    );
  });

async function runNetworkTest(
  ctx: Context,
  options: {
    prod?: boolean | undefined;
    previewName?: string | undefined;
    deploymentName?: string | undefined;
    url?: string | undefined;
    adminKey?: string | undefined;
    ipFamily?: string;
  },
) {
  showSpinner(ctx, "Performing network test...");
  const deploymentSelection = deploymentSelectionFromOptions(options);
  const url = await loadUrl(ctx, deploymentSelection);

  // First, check DNS to see if we can resolve the URL's hostname.
  await checkDns(ctx, url);

  // Second, check to see if we can open a TCP connection to the hostname.
  await checkTcp(ctx, url, options.ipFamily ?? "auto");

  // Fourth, do a simple HTTPS request and check that we receive a 200.
  await checkHttp(ctx, url);

  // Fifth, check a small echo request, much smaller than most networks' MTU.
  await checkEcho(ctx, url, 128);

  // Finally, try a few large echo requests, much larger than most networks' MTU.
  await checkEcho(ctx, url, 4 * 1024 * 1024);
  await checkEcho(ctx, url, 64 * 1024 * 1024);

  logFinishedStep(ctx, "Network test passed.");
}

async function loadUrl(
  ctx: Context,
  deploymentSelection: DeploymentSelection,
): Promise<string> {
  // Try to fetch the URL following the usual paths, but special case the
  // `--url` argument in case the developer doesn't have network connectivity.
  let url: string;
  if (
    deploymentSelection.kind === "urlWithAdminKey" ||
    deploymentSelection.kind === "urlWithLogin"
  ) {
    url = deploymentSelection.url;
  } else {
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );
    url = credentials.url;
  }
  logMessage(ctx, `${chalk.green(`✔`)} Project URL: ${url}`);
  return url;
}

async function checkDns(ctx: Context, url: string) {
  try {
    const hostname = new URL("/", url).hostname;
    const start = performance.now();
    type DnsResult = { duration: number; address: string; family: number };
    const result = await new Promise<DnsResult>((resolve, reject) => {
      dns.lookup(hostname, (err, address, family) => {
        if (err) {
          reject(err);
        } else {
          resolve({ duration: performance.now() - start, address, family });
        }
      });
    });
    logMessage(
      ctx,
      `${chalk.green(`✔`)} OK: DNS lookup => ${result.address}:${
        ipFamilyNames[result.family as keyof typeof ipFamilyNames]
      } (${formatDuration(result.duration)})`,
    );
  } catch (e: any) {
    logFailure(ctx, chalk.red(`FAIL: DNS lookup (${e})`));
    return ctx.crash(1, "transient");
  }
}

async function checkTcp(ctx: Context, urlString: string, ipFamilyOpt: string) {
  const url = new URL(urlString);
  if (url.protocol === "http:") {
    const port = Number.parseInt(url.port || "80");
    await checkTcpHostPort(ctx, url.hostname, port, ipFamilyOpt);
  } else if (url.protocol === "https:") {
    const port = Number.parseInt(url.port || "443");
    await checkTcpHostPort(ctx, url.hostname, port, ipFamilyOpt);
    // If we didn't specify a port, also try port 80.
    if (!url.port) {
      await checkTcpHostPort(ctx, url.hostname, 80, ipFamilyOpt);
    }
  } else {
    // eslint-disable-next-line no-restricted-syntax
    throw new Error(`Unknown protocol: ${url.protocol}`);
  }
}

async function checkTcpHostPort(
  ctx: Context,
  host: string,
  port: number,
  ipFamilyOpt: string,
) {
  const ipFamily = ipFamilyNumbers[ipFamilyOpt as keyof typeof ipFamilyNumbers];
  const tcpString =
    `TCP` + (ipFamilyOpt === "auto" ? "" : `/${ipFamilyOpt} ${host}:${port}`);
  try {
    const start = performance.now();
    const duration = await new Promise<number>((resolve, reject) => {
      const socket = net.connect(
        {
          host,
          port,
          noDelay: true,
          family: ipFamily,
        },
        () => resolve(performance.now() - start),
      );
      socket.on("error", (e) => reject(e));
    });
    logMessage(
      ctx,
      `${chalk.green(`✔`)} OK: ${tcpString} connect (${formatDuration(
        duration,
      )})`,
    );
  } catch (e: any) {
    logFailure(ctx, chalk.red(`FAIL: ${tcpString} connect (${e})`));
    return ctx.crash(1, "transient");
  }
}

async function checkHttp(ctx: Context, urlString: string) {
  const url = new URL(urlString);
  const isHttps = url.protocol === "https:";
  if (isHttps) {
    url.protocol = "http:";
    url.port = "80";
    await checkHttpOnce(ctx, "HTTP", url.toString(), 301, false);
  }
  await checkHttpOnce(ctx, isHttps ? "HTTPS" : "HTTP", urlString, 200, true);
}

async function checkHttpOnce(
  ctx: Context,
  name: string,
  url: string,
  expectedStatus: number,
  allowRedirects: boolean,
) {
  try {
    const start = performance.now();
    // Be sure to use the same axios client we use elsewhere so we're actually
    // getting coverage of our network stack.
    const client = deploymentClient(url);
    const instanceNameUrl = new URL("/instance_name", url);
    // Set `maxRedirects` to 0 so our HTTP test doesn't try HTTPS.
    const resp = await client.get(instanceNameUrl.toString(), {
      maxRedirects: allowRedirects ? undefined : 0,
      validateStatus: (status) => {
        return status === expectedStatus;
      },
    });
    if (resp.status !== expectedStatus) {
      // eslint-disable-next-line no-restricted-syntax
      throw new Error(`Unexpected status code: ${resp.status}`);
    }
    const duration = performance.now() - start;
    logMessage(
      ctx,
      `${chalk.green(`✔`)} OK: ${name} check (${formatDuration(duration)})`,
    );
  } catch (e: any) {
    logFailure(ctx, chalk.red(`FAIL: ${name} check (${e})`));
    return ctx.crash(1, "transient");
  }
}

async function checkEcho(ctx: Context, url: string, size: number) {
  try {
    const start = performance.now();
    const client = deploymentClient(url, (err) => {
      logFailure(
        ctx,
        chalk.red(`FAIL: echo ${formatSize(size)} (${err}), retrying...`),
      );
    });
    const echoUrl = new URL(`/echo`, url);
    const data = crypto.randomBytes(size);
    const resp = await client.post(echoUrl.toString(), data, {
      responseType: "arraybuffer",
    });
    if (resp.status !== 200) {
      // eslint-disable-next-line no-restricted-syntax
      throw new Error(`Unexpected status code: ${resp.status}`);
    }
    // Check that the returned data is equal.
    const respData = Buffer.from(resp.data);
    if (!data.equals(respData)) {
      // eslint-disable-next-line no-restricted-syntax
      throw new Error(`Response data mismatch`);
    }
    const duration = performance.now() - start;
    const bytesPerSecond = size / (duration / 1000);
    logMessage(
      ctx,
      `${chalk.green(`✔`)} OK: echo ${formatSize(size)} (${formatDuration(
        duration,
      )}, ${formatSize(bytesPerSecond)}/s)`,
    );
  } catch (e: any) {
    logFailure(ctx, chalk.red(`FAIL: echo ${formatSize(size)} (${e})`));
    return ctx.crash(1, "transient");
  }
}

export async function withTimeout<T>(
  ctx: Context,
  name: string,
  timeoutMs: number,
  f: Promise<T>,
) {
  let timer: NodeJS.Timeout | null = null;
  try {
    type TimeoutPromise = { kind: "ok"; result: T } | { kind: "timeout" };
    const result = await Promise.race<TimeoutPromise>([
      f.then((r) => {
        return { kind: "ok", result: r };
      }),
      new Promise((resolve) => {
        timer = setTimeout(() => {
          resolve({ kind: "timeout" as const });
          timer = null;
        }, timeoutMs);
      }),
    ]);
    if (result.kind === "ok") {
      return result.result;
    } else {
      logFailure(
        ctx,
        chalk.red(
          `FAIL: ${name} timed out after ${formatDuration(timeoutMs)}.`,
        ),
      );
      return await ctx.crash(1, "transient");
    }
  } finally {
    if (timer !== null) {
      clearTimeout(timer);
    }
  }
}
