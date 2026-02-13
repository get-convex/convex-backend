import {
  DefaultFunctionArgs,
  FunctionReference,
  httpRouter,
} from "convex/server";
import { ActionCtx, httpAction } from "./_generated/server";
import { internal } from "./_generated/api";
import { generateMessage } from "./util/message";
import { extractVersionFromHeader } from "./util/convexClientHeader";
import {
  OLD_CURSOR_RULES,
  shouldUseOldCursorRules,
} from "./util/oldCursorRules";
import { hashSha256 } from "./util/hash";

const http = httpRouter();

const COMMON_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, convex-client",
  "Cache-Control": "public, max-age=3600",
  Vary: "Convex-Client",
};

type VersionResponse = {
  message: string | null;
  cursorRulesHash: string | null;
};

http.route({
  path: "/v1/version",
  method: "GET",
  handler: httpAction(async (ctx, req) => {
    const convexClientHeader = req.headers.get("Convex-Client");
    const clientVersion = extractVersionFromHeader(convexClientHeader);

    const [npmVersionData, cursorRulesData] = await Promise.all([
      getCachedAndScheduleRefresh(ctx, internal.npm),
      getCursorRulesForVersion(ctx, clientVersion),
    ]);

    const message = npmVersionData
      ? generateMessage(npmVersionData, convexClientHeader)
      : null;

    return new Response(
      JSON.stringify({
        message,
        cursorRulesHash: cursorRulesData?.hash ?? null,
      } satisfies VersionResponse),
      {
        status: 200,
        headers: {
          ...COMMON_HEADERS,
          "Content-Type": "application/json",
        },
      },
    );
  }),
});

http.route({
  path: "/v1/cursor_rules",
  method: "GET",
  handler: httpAction(async (ctx, req) => {
    const convexClientHeader = req.headers.get("Convex-Client");
    const clientVersion = extractVersionFromHeader(convexClientHeader);

    const cursorRulesData = await getCursorRulesForVersion(ctx, clientVersion);

    if (!cursorRulesData) {
      return new Response("Can’t get the Cursor rules", {
        status: 500,
        headers: COMMON_HEADERS,
      });
    }

    return new Response(cursorRulesData.content, {
      status: 200,
      headers: {
        ...COMMON_HEADERS,
        "Content-Type": "text/plain",
      },
    });
  }),
});

// Handle CORS preflight requests
http.route({
  path: "/v1/version",
  method: "OPTIONS",
  handler: httpAction(async () => {
    return new Response(null, {
      status: 200,
      headers: COMMON_HEADERS,
    });
  }),
});

http.route({
  path: "/v1/cursor_rules",
  method: "OPTIONS",
  handler: httpAction(async () => {
    return new Response(null, {
      status: 200,
      headers: COMMON_HEADERS,
    });
  }),
});

http.route({
  path: "/v1/local_backend_version",
  method: "GET",
  handler: httpAction(async (ctx) => {
    const localBackendVersionData = await getCachedAndScheduleRefresh(
      ctx,
      internal.localBackend,
    );

    if (!localBackendVersionData) {
      return new Response("Failed to get local backend version", {
        status: 500,
        headers: COMMON_HEADERS,
      });
    }

    return new Response(
      JSON.stringify({
        version: localBackendVersionData.version,
      }),
      {
        status: 200,
        headers: {
          ...COMMON_HEADERS,
          "Content-Type": "application/json",
        },
      },
    );
  }),
});

http.route({
  path: "/v1/local_backend_version",
  method: "OPTIONS",
  handler: httpAction(async () => {
    return new Response(null, {
      status: 200,
      headers: COMMON_HEADERS,
    });
  }),
});

/**
 * Get the latest cached value. If it’s stale, return it and schedule a refresh.
 *
 * If we have no current cached version, get the latest version and cache it.
 */
export async function getCachedAndScheduleRefresh<
  Doc extends { _creationTime: number },
>(
  ctx: ActionCtx,
  module: {
    getCached: FunctionReference<
      "query",
      "internal",
      DefaultFunctionArgs,
      Doc | null
    >;
    refresh: FunctionReference<
      "action",
      "internal",
      DefaultFunctionArgs,
      Doc | null
    >;
  },
) {
  const cached = await ctx.runQuery(module.getCached, {});
  if (!cached) {
    return await ctx.runAction(module.refresh, {});
  }

  return cached;
}

async function getCursorRulesForVersion(
  ctx: ActionCtx,
  clientVersion: string | null,
): Promise<{
  hash: string;
  content: string;
} | null> {
  if (shouldUseOldCursorRules(clientVersion)) {
    return {
      content: OLD_CURSOR_RULES,
      hash: await hashSha256(OLD_CURSOR_RULES),
    };
  }

  return await getCachedAndScheduleRefresh(ctx, internal.cursorRules);
}

export default http;
