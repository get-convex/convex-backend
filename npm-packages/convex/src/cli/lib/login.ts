import { errors, BaseClient, custom } from "openid-client";
import {
  globalConfigPath,
  rootDirectory,
  GlobalConfig,
  getAuthHeaderForBigBrain,
  bigBrainAPI,
  logAndHandleFetchError,
  throwingFetch,
} from "./utils/utils.js";
import open from "open";
import chalk from "chalk";
import { provisionHost } from "./config.js";
import { version } from "../version.js";
import {
  Context,
  changeSpinner,
  logError,
  logFailure,
  logFinishedStep,
  logMessage,
  logOutput,
  showSpinner,
} from "../../bundler/context.js";
import { Issuer } from "openid-client";
import { hostname } from "os";
import { execSync } from "child_process";
import os from "os";
import { promptString, promptYesNo } from "./utils/prompts.js";

const SCOPE = "openid email profile";
/// This value was created long ago, and cannot be changed easily.
/// It's just a fixed string used for identifying the Auth0 token, so it's fine
/// and not user-facing.
const AUDIENCE = "https://console.convex.dev/api/";

// Per https://github.com/panva/node-openid-client/tree/main/docs#customizing
custom.setHttpOptionsDefaults({
  timeout: parseInt(process.env.OPENID_CLIENT_TIMEOUT || "10000"),
});

async function writeGlobalConfig(ctx: Context, config: GlobalConfig) {
  const dirName = rootDirectory();
  ctx.fs.mkdir(dirName, { allowExisting: true });
  const path = globalConfigPath();
  try {
    ctx.fs.writeUtf8File(path, JSON.stringify(config));
  } catch (err) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      errForSentry: err,
      printedMessage: chalk.red(
        `Failed to write auth config to ${path} with error: ${err as any}`,
      ),
    });
  }
  logFinishedStep(ctx, `Saved credentials to ${formatPathForPrinting(path)}`);
}

function formatPathForPrinting(path: string) {
  const homedir = os.homedir();
  if (process.platform === "darwin" && path.startsWith(homedir)) {
    return path.replace(homedir, "~");
  }
  return path;
}

export async function checkAuthorization(
  ctx: Context,
  acceptOptIns: boolean,
): Promise<boolean> {
  const header = await getAuthHeaderForBigBrain(ctx);
  if (!header) {
    return false;
  }
  try {
    const resp = await fetch(`${provisionHost}/api/authorize`, {
      method: "HEAD",
      headers: {
        Authorization: header,
        "Convex-Client": `npm-cli-${version}`,
      },
    });
    // Don't throw an error if this request returns a non-200 status.
    // Big Brain responds with a variety of error codes -- 401 if the token is correctly-formed but not valid, and either 400 or 500 if the token is ill-formed.
    // We only care if this check returns a 200 code (so we can skip logging in again) -- any other errors should be silently skipped and we'll run the whole login flow again.
    if (resp.status !== 200) {
      return false;
    }
  } catch (e: any) {
    // This `catch` block should only be hit if a network error was encountered
    logError(
      ctx,
      `Unexpected error when authorizing - are you connected to the internet?`,
    );
    return await logAndHandleFetchError(ctx, e);
  }

  // Check that we have optin as well
  const shouldContinue = await optins(ctx, acceptOptIns);
  if (!shouldContinue) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: null,
    });
  }
  return true;
}

async function performDeviceAuthorization(
  ctx: Context,
  auth0Client: BaseClient,
  shouldOpen: boolean,
): Promise<string> {
  // Device authorization flow follows this guide: https://github.com/auth0/auth0-device-flow-cli-sample/blob/9f0f3b76a6cd56ea8d99e76769187ea5102d519d/cli.js
  // License: MIT License
  // Copyright (c) 2019 Auth0 Samples
  /*
  The MIT License (MIT)

  Copyright (c) 2019 Auth0 Samples

  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software and associated documentation files (the "Software"), to deal
  in the Software without restriction, including without limitation the rights
  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies of the Software, and to permit persons to whom the Software is
  furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in all
  copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
  SOFTWARE.
  */

  // Device Authorization Request - https://tools.ietf.org/html/rfc8628#section-3.1
  // Get authentication URL
  let handle;
  try {
    handle = await auth0Client.deviceAuthorization({
      scope: SCOPE,
      audience: AUDIENCE,
    });
  } catch {
    // We couldn't get verification URL from Auth0, proceed with manual auth
    return promptString(ctx, {
      message:
        "Open https://dashboard.convex.dev/auth, log in and paste the token here:",
    });
  }

  // Device Authorization Response - https://tools.ietf.org/html/rfc8628#section-3.2
  // Open authentication URL
  const { verification_uri_complete, user_code, expires_in } = handle;
  logMessage(
    ctx,
    `Visit ${verification_uri_complete} to finish logging in.\n` +
      `You should see the following code which expires in ${
        expires_in % 60 === 0
          ? `${expires_in / 60} minutes`
          : `${expires_in} seconds`
      }: ${user_code}`,
  );
  if (shouldOpen) {
    shouldOpen = await promptYesNo(ctx, {
      message: `Open the browser?`,
      default: true,
    });
  }

  if (shouldOpen) {
    showSpinner(
      ctx,
      `Opening ${verification_uri_complete} in your browser to log in...\n`,
    );
    try {
      await open(verification_uri_complete);
      changeSpinner(ctx, "Waiting for the confirmation...");
    } catch {
      logError(ctx, chalk.red(`Unable to open browser.`));
      changeSpinner(
        ctx,
        `Manually open ${verification_uri_complete} in your browser to log in.`,
      );
    }
  } else {
    showSpinner(
      ctx,
      `Open ${verification_uri_complete} in your browser to log in.`,
    );
  }

  // Device Access Token Request - https://tools.ietf.org/html/rfc8628#section-3.4
  // Device Access Token Response - https://tools.ietf.org/html/rfc8628#section-3.5
  try {
    const tokens = await handle.poll();
    if (typeof tokens.access_token === "string") {
      return tokens.access_token;
    } else {
      // Unexpected error
      // eslint-disable-next-line no-restricted-syntax
      throw Error("Access token is missing");
    }
  } catch (err: any) {
    switch (err.error) {
      case "access_denied": // end-user declined the device confirmation prompt, consent or rules failed
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: "Access denied.",
          errForSentry: err,
        });
      case "expired_token": // end-user did not complete the interaction in time
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: "Device flow expired.",
          errForSentry: err,
        });
      default: {
        const message =
          err instanceof errors.OPError
            ? `Error = ${err.error}; error_description = ${err.error_description}`
            : `Login failed with error: ${err}`;
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: message,
          errForSentry: err,
        });
      }
    }
  }
}

async function performPasswordAuthentication(
  ctx: Context,
  issuer: string,
  clientId: string,
  username: string,
  password: string,
): Promise<string> {
  // Unfortunately, `openid-client` doesn't support the resource owner password credentials flow so we need to manually send the requests.
  const options: Parameters<typeof throwingFetch>[1] = {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "password",
      username: username,
      password: password,
      scope: SCOPE,
      client_id: clientId,
      audience: AUDIENCE,
      // Note that there is no client secret provided, as Auth0 refuses to require it for untrusted apps.
    }),
  };

  try {
    const response = await throwingFetch(
      new URL("/oauth/token", issuer).href,
      options,
    );
    const data = await response.json();
    if (typeof data.access_token === "string") {
      return data.access_token;
    } else {
      // Unexpected error
      // eslint-disable-next-line no-restricted-syntax
      throw Error("Access token is missing");
    }
  } catch (err: any) {
    logFailure(ctx, `Password flow failed: ${err}`);
    if (err.response) {
      logError(ctx, chalk.red(`${JSON.stringify(err.response.data)}`));
    }
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      errForSentry: err,
      printedMessage: null,
    });
  }
}

export async function performLogin(
  ctx: Context,
  {
    overrideAuthUrl,
    overrideAuthClient,
    overrideAuthUsername,
    overrideAuthPassword,
    overrideAccessToken,
    open,
    acceptOptIns,
    dumpAccessToken,
    deviceName: deviceNameOverride,
  }: {
    overrideAuthUrl?: string;
    overrideAuthClient?: string;
    overrideAuthUsername?: string;
    overrideAuthPassword?: string;
    overrideAccessToken?: string;
    // default `true`
    open?: boolean;
    // default `true`
    acceptOptIns?: boolean;
    dumpAccessToken?: boolean;
    deviceName?: string;
  } = {},
) {
  // Get access token from big-brain
  // Default the device name to the hostname, but allow the user to change this if the terminal is interactive.
  // On Macs, the `hostname()` may be a weirdly-truncated form of the computer name. Attempt to read the "real" name before falling back to hostname.
  let deviceName = deviceNameOverride ?? "";
  if (!deviceName && process.platform === "darwin") {
    try {
      deviceName = execSync("scutil --get ComputerName").toString().trim();
    } catch {
      // Just fall back to the hostname default below.
    }
  }
  if (!deviceName) {
    deviceName = hostname();
  }
  if (!deviceNameOverride) {
    logMessage(
      ctx,
      chalk.bold(`Welcome to developing with Convex, let's get you logged in.`),
    );
    deviceName = await promptString(ctx, {
      message: "Device name:",
      default: deviceName,
    });
  }

  const issuer = overrideAuthUrl ?? "https://auth.convex.dev";
  const auth0 = await Issuer.discover(issuer);
  const clientId = overrideAuthClient ?? "HFtA247jp9iNs08NTLIB7JsNPMmRIyfi";
  const auth0Client = new auth0.Client({
    client_id: clientId,
    token_endpoint_auth_method: "none",
    id_token_signed_response_alg: "RS256",
  });

  let accessToken: string;
  if (overrideAccessToken) {
    accessToken = overrideAccessToken;
  } else if (overrideAuthUsername && overrideAuthPassword) {
    accessToken = await performPasswordAuthentication(
      ctx,
      issuer,
      clientId,
      overrideAuthUsername,
      overrideAuthPassword,
    );
  } else {
    accessToken = await performDeviceAuthorization(
      ctx,
      auth0Client,
      open ?? true,
    );
  }

  if (dumpAccessToken) {
    logOutput(ctx, `${accessToken}`);
    return await ctx.crash({
      exitCode: 0,
      errorType: "fatal",
      printedMessage: null,
    });
  }

  interface AuthorizeArgs {
    authnToken: string;
    deviceName: string;
  }
  const authorizeArgs: AuthorizeArgs = {
    authnToken: accessToken,
    deviceName: deviceName,
  };
  const data = await bigBrainAPI({
    ctx,
    method: "POST",
    url: "authorize",
    data: authorizeArgs,
  });
  const globalConfig = { accessToken: data.accessToken };
  try {
    await writeGlobalConfig(ctx, globalConfig);
  } catch (err: unknown) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      errForSentry: err,
      printedMessage: null,
    });
  }

  // Do opt in to TOS and Privacy Policy stuff
  const shouldContinue = await optins(ctx, acceptOptIns ?? false);
  if (!shouldContinue) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: null,
    });
  }
}

/// There are fields like version, but we keep them opaque
type OptIn = Record<string, unknown>;

type OptInToAccept = {
  optIn: OptIn;
  message: string;
};

type AcceptOptInsArgs = {
  optInsAccepted: OptIn[];
};

// Returns whether we can proceed or not.
async function optins(ctx: Context, acceptOptIns: boolean): Promise<boolean> {
  const data = await bigBrainAPI({
    ctx,
    method: "POST",
    url: "check_opt_ins",
  });
  if (data.optInsToAccept.length === 0) {
    return true;
  }
  for (const optInToAccept of data.optInsToAccept) {
    const confirmed =
      acceptOptIns ||
      (await promptYesNo(ctx, {
        message: optInToAccept.message,
      }));
    if (!confirmed) {
      logFailure(ctx, "Please accept the Terms of Service to use Convex.");
      return Promise.resolve(false);
    }
  }

  const optInsAccepted = data.optInsToAccept.map((o: OptInToAccept) => o.optIn);
  const args: AcceptOptInsArgs = { optInsAccepted };
  await bigBrainAPI({ ctx, method: "POST", url: "accept_opt_ins", data: args });
  return true;
}
