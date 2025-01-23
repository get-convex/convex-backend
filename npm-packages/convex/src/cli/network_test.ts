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
import {
  bareDeploymentFetch,
  formatDuration,
  formatSize,
  ThrowingFetchError,
} from "./lib/utils/utils.js";
import chalk from "chalk";

const ipFamilyNumbers = { ipv4: 4, ipv6: 6, auto: 0 } as const;
const ipFamilyNames = { 4: "ipv4", 6: "ipv6", 0: "auto" } as const;

export const networkTest = new Command("network-test")
  .description("Run a network test to Convex's servers")
  .allowExcessArguments(false)
  .addOption(
    new Option(
      "--timeout <timeout>",
      "Timeout in seconds for the network test (default: 30).",
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
      "--speed-test",
      "Perform a large echo test to measure network speed.",
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
    const ctx = oneoffContext();
    const timeoutSeconds = options.timeout
      ? Number.parseFloat(options.timeout)
      : 30;
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
    speedTest?: boolean;
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

  // Finally, try a large echo request, much larger than most networks' MTU.
  await checkEcho(ctx, url, 4 * 1024 * 1024);
  // Also do a 64MiB echo test if the user has requested a speed test.
  if (options.speedTest) {
    await checkEcho(ctx, url, 64 * 1024 * 1024);
  }

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
    return ctx.crash({
      exitCode: 1,
      errorType: "transient",
      printedMessage: `FAIL: DNS lookup (${e})`,
    });
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
    return ctx.crash({
      exitCode: 1,
      errorType: "transient",
      printedMessage: `FAIL: ${tcpString} connect (${e})`,
    });
  }
}

async function checkHttp(ctx: Context, urlString: string) {
  const url = new URL(urlString);
  const isHttps = url.protocol === "https:";
  if (isHttps) {
    url.protocol = "http:";
    url.port = "80";
    await checkHttpOnce(ctx, "HTTP", url.toString(), false);
  }
  await checkHttpOnce(ctx, isHttps ? "HTTPS" : "HTTP", urlString, true);
}

// Be sure to test this function against *prod* (with both HTTP & HTTPS) when
// making changes.
async function checkHttpOnce(
  ctx: Context,
  name: string,
  url: string,
  allowRedirects: boolean,
) {
  const start = performance.now();
  try {
    // Be sure to use the same `deploymentFetch` we use elsewhere so we're actually
    // getting coverage of our network stack.
    const fetch = bareDeploymentFetch(ctx, { deploymentUrl: url });
    const instanceNameUrl = new URL("/instance_name", url);
    const resp = await fetch(instanceNameUrl.toString(), {
      redirect: allowRedirects ? "follow" : "manual",
    });
    if (resp.status !== 200) {
      // eslint-disable-next-line no-restricted-syntax
      throw new Error(`Unexpected status code: ${resp.status}`);
    }
  } catch (e: any) {
    // Redirects return a 301, which causes `bareDeploymentFetch` to throw an
    // ThrowingFetchError. Catch that here and succeed if we're not following
    // redirects.
    const isOkayRedirect =
      !allowRedirects &&
      e instanceof ThrowingFetchError &&
      e.response.status === 301;
    if (!isOkayRedirect) {
      return ctx.crash({
        exitCode: 1,
        errorType: "transient",
        printedMessage: `FAIL: ${name} check (${e})`,
      });
    }
  }
  const duration = performance.now() - start;
  logMessage(
    ctx,
    `${chalk.green(`✔`)} OK: ${name} check (${formatDuration(duration)})`,
  );
}

async function checkEcho(ctx: Context, url: string, size: number) {
  try {
    const start = performance.now();
    const fetch = bareDeploymentFetch(ctx, {
      deploymentUrl: url,
      onError: (err) => {
        logFailure(
          ctx,
          chalk.red(`FAIL: echo ${formatSize(size)} (${err}), retrying...`),
        );
      },
    });
    const echoUrl = new URL(`/echo`, url);
    const data = crypto.randomBytes(size);
    const resp = await fetch(echoUrl.toString(), {
      body: data,
      method: "POST",
    });
    if (resp.status !== 200) {
      // eslint-disable-next-line no-restricted-syntax
      throw new Error(`Unexpected status code: ${resp.status}`);
    }
    const respData = await resp.arrayBuffer();
    if (!data.equals(Buffer.from(respData))) {
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
    return ctx.crash({
      exitCode: 1,
      errorType: "transient",
      printedMessage: `FAIL: echo ${formatSize(size)} (${e})`,
    });
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
      return await ctx.crash({
        exitCode: 1,
        errorType: "transient",
        printedMessage: `FAIL: ${name} timed out after ${formatDuration(timeoutMs)}.`,
      });
    }
  } finally {
    if (timer !== null) {
      clearTimeout(timer);
    }
  }
}
