import { randomUUID } from "crypto";
import { z } from "zod";

import { DeploymentMetadata, UserIdentity } from "convex/server";
import { ExecutionContext, SyscallStats } from "./executor";
import { ConvexError, JSONValue } from "convex/values";
import { UdfPath } from "./convex";

const MAX_PENDING_SYSCALLS = 1000;

const STATUS_CODE_BAD_REQUEST = 400;
// Special custom 5xx HTTP status code to mean that the UDF returned an error.
//
// Must match the constant of the same name in Rust.
const STATUS_CODE_UDF_FAILED = 560;

// Retry settings for transient (5xx) failures when an action calls back into
// the backend to run a query or mutation.
const CALLBACK_MAX_ATTEMPTS = 5;
const CALLBACK_INITIAL_BACKOFF_MS = 1000;
const CALLBACK_MAX_BACKOFF_MS = 20000;

function callbackBackoffMs(attempt: number): number {
  const base = Math.min(
    CALLBACK_MAX_BACKOFF_MS,
    CALLBACK_INITIAL_BACKOFF_MS * 2 ** attempt,
  );
  // Full jitter to avoid synchronized retries across concurrent callbacks.
  return Math.random() * base;
}

// A 5xx status other than STATUS_CODE_UDF_FAILED (which represents a real
// error thrown by the called function) is a transient backend/proxy failure
// that is safe to retry.
function isTransientStatus(status: number): boolean {
  return status >= 500 && status < 600 && status !== STATUS_CODE_UDF_FAILED;
}

const runFunctionArgs = z.object({
  name: z.optional(z.string()),
  reference: z.optional(z.string()),
  functionHandle: z.optional(z.string()),
  args: z.any(),
  version: z.string(),
});

const runFunctionReturn = z.union([
  z.object({
    status: z.literal("success"),
    value: z.any(),
  }),
  z.object({
    status: z.literal("error"),
    errorData: z.optional(z.any()),
    errorMessage: z.string(),
  }),
]);

const scheduleSchema = z.object({
  name: z.optional(z.string()),
  reference: z.optional(z.string()),
  functionHandle: z.optional(z.string()),
  ts: z.number(),
  args: z.any(),
  version: z.string(),
});

const storageGetSchema = z.object({
  storageId: z.string(),
  version: z.string(),
});

export type ScheduledJob = z.infer<typeof scheduleSchema>;

export interface Syscalls {
  syscall(op: string, jsonArgs: string): string;
  asyncSyscall(op: string, jsonArgs: string): Promise<string>;
  asyncJsSyscall(op: string, args: Record<string, any>): Promise<any>;

  assertNoPendingSyscalls(): void;
}

async function defaultHandleResponseError(
  response: Response,
  operationName: string,
): Promise<void> {
  if (response.status === STATUS_CODE_BAD_REQUEST) {
    const text = await response.text();
    throw new Error(`Invalid ${operationName} request: ${text}`);
  }
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Transient error while running ${operationName}: ${text}`);
  }
  return;
}

export class SyscallsImpl {
  udfPath: UdfPath;
  lambdaExecuteId: string;
  backendAddress: string;
  backendCallbackToken: string;
  authHeader: string | null;
  executionContext: ExecutionContext;

  // The `userIdentity` is determined from `authHeader`, but we only want to implement parsing in
  // Rust, so we'll unpack it there and send both to JS.
  userIdentity: UserIdentity | null;

  syscallTrace: Record<string, SyscallStats>;

  pendingSyscallCount: Record<string, number>;

  encodedParentTrace: string | null;

  deployment: DeploymentMetadata;

  // Identifies this action execution as a "session" for the purpose of making
  // the mutations it runs idempotent across client-side retries. Each mutation
  // callback gets a monotonically increasing `requestId` within this session.
  mutationSessionId: string;
  nextMutationRequestId: number;

  constructor(
    udfPath: UdfPath,
    lambdaExecuteId: string,
    backendAddress: string,
    backendCallbackToken: string,
    authHeader: string | null,
    userIdentity: UserIdentity | null,
    executionContext: ExecutionContext,
    encodedParentTrace: string | null,
    deployment: DeploymentMetadata,
  ) {
    this.udfPath = udfPath;
    this.lambdaExecuteId = lambdaExecuteId;
    this.backendAddress = backendAddress;
    this.backendCallbackToken = backendCallbackToken;
    this.authHeader = authHeader;
    this.userIdentity = userIdentity;
    this.syscallTrace = {};
    this.pendingSyscallCount = {};
    this.executionContext = executionContext;
    this.encodedParentTrace = encodedParentTrace;
    this.deployment = deployment;
    this.mutationSessionId = randomUUID();
    this.nextMutationRequestId = 0;
  }

  async actionCallback<ResponseValidator extends z.ZodType>(args: {
    version: string;
    body: Record<string, any>;
    path: string;
    operationName: string;
    handleResponseErrorCode?: (
      response: Response,
      operationName: string,
    ) => Promise<void>;
    responseValidator: ResponseValidator;
    // Whether to retry transient (5xx) failures. Only safe for read-only
    // queries and for mutations carrying a `mutationIdentifier` (which makes
    // the backend dedupe a retried mutation that already committed).
    retryTransient: boolean;
  }): Promise<z.infer<ResponseValidator>> {
    const headers = this.headers(args.version);
    const url = new URL(args.path, this.backendAddress);
    const body = JSON.stringify(args.body);
    const maxAttempts = args.retryTransient ? CALLBACK_MAX_ATTEMPTS : 1;
    let response: Response;
    let attempt = 0;
    for (;;) {
      response = await fetch(url, { body, method: "POST", headers });
      attempt += 1;
      if (!isTransientStatus(response.status) || attempt >= maxAttempts) {
        break;
      }
      await new Promise((resolve) =>
        setTimeout(resolve, callbackBackoffMs(attempt - 1)),
      );
    }
    const errorHandler =
      args.handleResponseErrorCode ?? defaultHandleResponseError;
    await errorHandler(response, args.operationName);
    try {
      const body = await response.json();
      const parsedBody = args.responseValidator.parse(body);
      return parsedBody;
    } catch {
      // This probably represents an error on our side where we're returning the wrong
      // response type, and should ideally never happen. Throw a generic error when
      // it does happen though.
      throw new Error(`Transient error while running ${args.operationName}`);
    }
  }

  headers(version: string): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "Convex-Client": `actions-${version}`,
      "Convex-Action-Callback-Token": this.backendCallbackToken,
      "Convex-Action-Function-Name": `${this.udfPath.canonicalizedPath}:${this.udfPath.function}`,
    };
    if (this.executionContext.parentScheduledJob !== null) {
      headers["Convex-Parent-Scheduled-Job"] =
        this.executionContext.parentScheduledJob;
    }
    if (this.executionContext.parentScheduledJobComponentId !== null) {
      headers["Convex-Parent-Scheduled-Job-Component-Id"] =
        this.executionContext.parentScheduledJobComponentId;
    }
    headers["Convex-Request-Id"] = this.executionContext.requestId;
    if (this.executionContext.ip !== null) {
      headers["Convex-Request-Client-Ip"] = this.executionContext.ip;
    }
    if (this.executionContext.userAgent !== null) {
      headers["Convex-Request-Client-User-Agent"] =
        this.executionContext.userAgent;
    }
    if (this.executionContext.executionId !== undefined) {
      headers["Convex-Execution-Id"] = this.executionContext.executionId;
    }
    if (this.executionContext.isRoot !== undefined) {
      headers["Convex-Root-Request"] = this.executionContext.isRoot.toString();
    }
    if (this.authHeader !== null) {
      headers["Authorization"] = this.authHeader;
    }
    if (this.encodedParentTrace !== null) {
      headers["Convex-Encoded-Parent-Trace"] = this.encodedParentTrace;
    }
    return headers;
  }

  validateArgs<ArgValidator extends z.ZodType>(
    jsonArgs: string,
    argValidator: ArgValidator,
    operationName: string,
    requireRequestId: boolean = true,
  ): z.infer<ArgValidator> {
    const args = JSON.parse(jsonArgs);
    if (requireRequestId) {
      // TODO(CX-5733): Rename requestId to lambdaExecuteId in callers and here.
      this.validateLambdaExecuteId(args.requestId);
    }
    delete args.requestId;
    try {
      const parsedArgs = argValidator.parse(args);
      return parsedArgs;
    } catch {
      throw new Error(
        `Invalid ${operationName} request with args ${JSON.stringify(args)}`,
      );
    }
  }

  validateLambdaExecuteId(lambdaExecuteId: string) {
    if (!lambdaExecuteId) {
      throw new Error(
        "Invalid syscall. The Convex syscalls are for internal Convex use and should not be " +
          "called directly.",
      );
    }

    if (lambdaExecuteId !== this.lambdaExecuteId) {
      throw new Error(
        "Leftover state detected. This typically happens if there are dangling promises from a " +
          "previous request. Did you forget to await your promises?",
      );
    }
  }

  assertNoPendingSyscalls() {
    for (const [syscallName, count] of Object.entries(
      this.pendingSyscallCount,
    )) {
      if (count > 0) {
        let operationName = syscallName;
        if (operationName.startsWith("1.0/")) {
          operationName = operationName.slice("1.0/".length);
        }
        if (operationName.startsWith("actions/")) {
          operationName = operationName.slice("actions/".length);
        }
        if (operationName.startsWith("storage")) {
          operationName = "storage " + operationName.slice("storage".length);
        }
        console.warn(
          `You have an outstanding ${operationName} call. ` +
            `Operations should be awaited or they might not run. ` +
            `Not awaiting promises might result in unexpected failures. ` +
            `See https://docs.convex.dev/functions/actions#dangling-promises for more information.`,
        );
      }
    }
  }

  syscall(op: string, _jsonArgs: string): string {
    // Note: We don't call validateArgs at the top since we want to throw an error
    // for unknown / isolate syscalls being called before we validate the requestId.
    switch (op) {
      case "1.0/queryCleanup":
      case "1.0/queryStream":
      case "1.0/db/normalizeId":
        throw new Error(
          "The Convex database object is being used outside of a Convex query or mutation. Did" +
            "you mean to use `ctx.query` or `ctx.mutation` to access the database?",
        );
      default:
        throw new Error(`Unknown operation ${op}`);
    }
  }

  async asyncSyscall(op: string, jsonArgs: string): Promise<string> {
    const start = performance.now();
    let isSuccess = true;

    const totalPendingSyscalls = Object.values(this.pendingSyscallCount).reduce(
      (a, b) => a + b,
      0,
    );
    if (totalPendingSyscalls >= MAX_PENDING_SYSCALLS) {
      throw new Error(
        "Too many concurrent operations. See https://docs.convex.dev/functions/actions#limits for more information.",
      );
    }

    if (!this.pendingSyscallCount[op]) {
      this.pendingSyscallCount[op] = 1;
    } else {
      this.pendingSyscallCount[op] += 1;
    }

    // Note: We don't call validateArgs at the top since we want to throw an error
    // for unknown / isolate syscalls being called before we validate the requestId.
    try {
      // Note: It's important to `await` the promises within the try block since
      // we want to catch their exceptions below. Returning the promise directly
      // doesn't throw the exception in this function.
      switch (op) {
        case "1.0/actions/query": {
          return JSON.stringify(await this.syscallQuery(jsonArgs));
        }
        case "1.0/actions/mutation": {
          return JSON.stringify(await this.syscallMutation(jsonArgs));
        }
        case "1.0/actions/action": {
          return JSON.stringify(await this.syscallAction(jsonArgs));
        }
        case "1.0/actions/vectorSearch": {
          return JSON.stringify(await this.syscallVectorSearch(jsonArgs));
        }
        case "1.0/schedule":
          throw new Error(
            "The mutation scheduler is being used outside of a Convex mutation. Did" +
              "you mean to use `ctx.mutation` to access the database?",
          );
        case "actions/schedule":
        case "1.0/actions/schedule":
          return JSON.stringify(await this.syscallSchedule(jsonArgs));
        case "1.0/actions/cancel_job":
          return JSON.stringify(await this.syscallCancelJob(jsonArgs));
        case "1.0/getUserIdentity":
          return JSON.stringify(this.syscallGetUserIdentity(jsonArgs));
        case "1.0/storageGenerateUploadUrl": {
          return JSON.stringify(
            await this.syscallStorageGenerateUploadUrl(jsonArgs),
          );
        }
        case "1.0/storageGetUrl":
          return JSON.stringify(await this.syscallStorageGetUrl(jsonArgs));
        case "1.0/storageGetMetadata":
          return JSON.stringify(await this.syscallStorageGetMetadata(jsonArgs));
        case "1.0/storageDelete":
          return JSON.stringify(await this.syscallStorageDelete(jsonArgs));
        case "1.0/createFunctionHandle":
          return JSON.stringify(
            await this.syscallCreateFunctionHandle(jsonArgs),
          );
        case "1.0/getFunctionMetadata":
          return JSON.stringify({
            name: `${stripExtension(this.udfPath.canonicalizedPath)}:${this.udfPath.function}`,
            // TODO: plumb componentPath through ExecuteRequest when we
            // support node actions for components.
            componentPath: "",
          });
        case "1.0/getDeploymentMetadata":
          return JSON.stringify(this.deployment);
        case "1.0/getRequestMetadata": {
          // Expose the raw auth JWT the request was authenticated with, if any.
          // Only `Bearer` (user) auth carries a JWT; admin keys (`Convex ...`)
          // and unauthenticated requests have no token.
          const authToken =
            this.authHeader !== null &&
            this.authHeader.slice(0, 7).toLowerCase() === "bearer "
              ? this.authHeader.slice(7).trim()
              : null;
          return JSON.stringify({
            ip: this.executionContext.ip,
            userAgent: this.executionContext.userAgent,
            requestId: this.executionContext.requestId,
            // The top-level scheduled function and all of its descendants report
            // the scheduled function's id (`parentScheduledJob` is propagated
            // down the call tree). It is null when the function was not
            // scheduled.
            scheduledFunctionId: this.executionContext.parentScheduledJob,
            authToken,
          });
        }
        case "1.0/auditLog":
          return JSON.stringify(await this.syscallAuditLog(jsonArgs));
        default:
          throw new Error(`Unknown operation ${op}`);
      }
    } catch (e: any) {
      isSuccess = false;
      throw e;
    } finally {
      this.pendingSyscallCount[op] -= 1;
      if (!this.syscallTrace[op]) {
        this.syscallTrace[op] = {
          invocations: 0,
          errors: 0,
          totalDurationMs: 0,
        };
      }
      const trace = this.syscallTrace[op]!;
      trace.invocations += 1;
      if (!isSuccess) {
        trace.errors += 1;
      }
      trace.totalDurationMs += performance.now() - start;
    }
  }

  async asyncJsSyscall(op: string, args: Record<string, any>): Promise<any> {
    switch (op) {
      case "storage/storeBlob":
        return this.syscallStoreBlob(args);
      case "storage/getBlob":
        return this.syscallGetBlob(args);
      default:
        throw new Error(`Unknown operation ${op}`);
    }
  }

  async syscallQuery(rawArgs: string): Promise<JSONValue> {
    const operationName = "query";
    const queryArgs = this.validateArgs(
      rawArgs,
      runFunctionArgs,
      operationName,
    );
    const handleResponseErrorCode = async (response: Response) => {
      if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
        const text = await response.text();
        throw new Error(text);
      }
    };
    const queryResult = await this.actionCallback({
      version: queryArgs.version,
      body: {
        path: queryArgs.name,
        reference: queryArgs.reference,
        functionHandle: queryArgs.functionHandle,
        args: queryArgs.args,
      },
      path: "/api/actions/query",
      operationName,
      responseValidator: runFunctionReturn,
      handleResponseErrorCode,
      // Queries are read-only, so retrying a transient failure is always safe.
      retryTransient: true,
    });
    switch (queryResult.status) {
      case "success":
        return queryResult.value;
      case "error":
        if (queryResult.errorData !== undefined) {
          throw forwardErrorData(
            queryResult.errorData,
            new ConvexError(queryResult.errorMessage),
          );
        }
        throw new Error(queryResult.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(queryResult)}`);
    }
  }

  async syscallMutation(rawArgs: string): Promise<JSONValue> {
    const operationName = "mutation";
    const mutationArgs = this.validateArgs(
      rawArgs,
      runFunctionArgs,
      operationName,
    );
    const handleResponseErrorCode = async (response: Response) => {
      if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
        const text = await response.text();
        throw new Error(text);
      }
    };
    // Capture the identifier once, before any retry, so every retry of this
    // mutation reuses it. The backend records the outcome atomically with the
    // mutation, so a retry that lands after the original already committed
    // returns the recorded result instead of running the mutation again.
    const mutationIdentifier = {
      sessionId: this.mutationSessionId,
      requestId: this.nextMutationRequestId++,
    };
    const mutationResult = await this.actionCallback({
      version: mutationArgs.version,
      body: {
        path: mutationArgs.name,
        reference: mutationArgs.reference,
        functionHandle: mutationArgs.functionHandle,
        args: mutationArgs.args,
        mutationIdentifier,
      },
      path: "/api/actions/mutation",
      operationName,
      responseValidator: runFunctionReturn,
      handleResponseErrorCode,
      // Safe to retry because `mutationIdentifier` makes the mutation
      // idempotent on the backend.
      retryTransient: true,
    });
    switch (mutationResult.status) {
      case "success":
        return mutationResult.value;
      case "error":
        if (mutationResult.errorData !== undefined) {
          throw forwardErrorData(
            mutationResult.errorData,
            new ConvexError(mutationResult.errorMessage),
          );
        }
        throw new Error(mutationResult.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(mutationResult)}`);
    }
  }

  async syscallAction(rawArgs: string): Promise<string> {
    const operationName = "action";
    const actionArgs = this.validateArgs(
      rawArgs,
      runFunctionArgs,
      operationName,
    );
    const handleResponseErrorCode = async (response: Response) => {
      if (!response.ok && response.status !== STATUS_CODE_UDF_FAILED) {
        const text = await response.text();
        throw new Error(text);
      }
    };
    const actionResult = await this.actionCallback({
      version: actionArgs.version,
      body: {
        path: actionArgs.name,
        reference: actionArgs.reference,
        functionHandle: actionArgs.functionHandle,
        args: actionArgs.args,
      },
      path: "/api/actions/action",
      operationName,
      responseValidator: runFunctionReturn,
      handleResponseErrorCode,
      // Actions are not idempotent, so a retried action could run twice.
      retryTransient: false,
    });
    switch (actionResult.status) {
      case "success":
        return actionResult.value;
      case "error":
        if (actionResult.errorData !== undefined) {
          throw forwardErrorData(
            actionResult.errorData,
            new ConvexError(actionResult.errorMessage),
          );
        }
        throw new Error(actionResult.errorMessage);
      default:
        throw new Error(`Invalid response: ${JSON.stringify(actionResult)}`);
    }
  }

  async syscallVectorSearch(rawArgs: string): Promise<JSONValue> {
    const vectorSearchSchema = z.object({
      query: z.any(),
      version: z.string(),
    });
    const vectorSearchReturn = z.object({
      results: z.array(z.any()),
    });
    const operationName = "vector search";
    const vectorSearchArgs = this.validateArgs(
      rawArgs,
      vectorSearchSchema,
      operationName,
    );
    return this.actionCallback({
      version: vectorSearchArgs.version,
      body: { query: vectorSearchArgs.query },
      path: "/api/actions/vector_search",
      operationName,
      responseValidator: vectorSearchReturn,
      retryTransient: false,
    });
  }

  async syscallSchedule(rawArgs: string): Promise<JSONValue> {
    const scheduleReturn = z.object({
      jobId: z.string(),
    });
    const operationName = "schedule";
    const scheduleArgs = this.validateArgs(
      rawArgs,
      scheduleSchema,
      operationName,
    );
    const { jobId } = await this.actionCallback({
      version: scheduleArgs.version,
      body: {
        reference: scheduleArgs.reference,
        functionHandle: scheduleArgs.functionHandle,
        udfPath: scheduleArgs.name,
        udfArgs: scheduleArgs.args,
        scheduledTs: scheduleArgs.ts,
      },
      path: "/api/actions/schedule_job",
      operationName,
      responseValidator: scheduleReturn,
      retryTransient: false,
    });
    return jobId;
  }

  async syscallCancelJob(rawArgs: string): Promise<JSONValue> {
    const cancelJobSchema = z.object({
      id: z.string(),
      version: z.string(),
    });
    const operationName = "cancel job";
    const args = this.validateArgs(rawArgs, cancelJobSchema, operationName);
    await this.actionCallback({
      version: args.version,
      body: {
        id: args.id,
      },
      path: "/api/actions/cancel_job",
      operationName,
      responseValidator: z.any(),
      retryTransient: false,
    });
    return null;
  }

  syscallGetUserIdentity(rawArgs: string): JSONValue {
    this.validateArgs(rawArgs, z.any(), "get user identity");
    return this.userIdentity as JSONValue;
  }

  async syscallStorageGenerateUploadUrl(rawArgs: string): Promise<JSONValue> {
    const storageGenerateUploadUrlArgs = z.object({
      version: z.string(),
    });
    const operationName = "generate upload url";
    const args = this.validateArgs(
      rawArgs,
      storageGenerateUploadUrlArgs,
      operationName,
    );
    return this._storageGenerateUploadUrl(args.version);
  }

  async _storageGenerateUploadUrl(version: string): Promise<string> {
    const storageGenerateUploadUrlReturn = z.object({
      url: z.string(),
    });
    const operationName = "generate upload url";
    const result = await this.actionCallback({
      version,
      body: {},
      path: "/api/actions/storage_generate_upload_url",
      operationName,
      responseValidator: storageGenerateUploadUrlReturn,
      retryTransient: false,
    });
    return result.url;
  }

  async syscallStorageGetUrl(rawArgs: string): Promise<JSONValue> {
    const operationName = "storage get url";
    const args = this.validateArgs(rawArgs, storageGetSchema, operationName);
    return this._storageGetUrl({
      version: args.version,
      storageId: args.storageId,
    });
  }

  async _storageGetUrl(args: {
    version: string;
    storageId: string;
  }): Promise<string | null> {
    const storageGetUrlReturn = z.object({
      url: z.union([z.string(), z.null()]),
    });
    const operationName = "storage get url";
    const result = await this.actionCallback({
      version: args.version,
      body: { storageId: args.storageId },
      path: "/api/actions/storage_get_url",
      operationName,
      responseValidator: storageGetUrlReturn,
      retryTransient: false,
    });
    return result.url;
  }

  async syscallStorageGetMetadata(rawArgs: string): Promise<JSONValue> {
    const operationName = "storage get metadata";
    const args = this.validateArgs(rawArgs, storageGetSchema, operationName);
    return this.actionCallback({
      version: args.version,
      body: { storageId: args.storageId },
      path: "/api/actions/storage_get_metadata",
      operationName,
      responseValidator: z.any(),
      retryTransient: false,
    });
  }

  async syscallStorageDelete(rawArgs: string): Promise<JSONValue> {
    const operationName = "storage delete";
    const args = this.validateArgs(rawArgs, storageGetSchema, operationName);
    return this.actionCallback({
      version: args.version,
      body: { storageId: args.storageId },
      path: "/api/actions/storage_delete",
      operationName,
      responseValidator: z.any(),
      retryTransient: false,
    });
  }

  async syscallAuditLog(rawArgs: string): Promise<JSONValue> {
    const auditLogSchema = z.object({
      body: z.record(z.string(), z.any()),
      version: z.string(),
    });
    const operationName = "audit log";
    const args = this.validateArgs(
      rawArgs,
      auditLogSchema,
      operationName,
      false,
    );
    return this.actionCallback({
      version: args.version,
      body: { body: args.body },
      path: "/api/actions/audit_log",
      operationName,
      responseValidator: z.any(),
      retryTransient: false,
    });
  }

  async syscallStoreBlob(args: Record<string, any>): Promise<any> {
    if (
      args["requestId"] === undefined ||
      args["blob"] === undefined ||
      args["version"] === undefined
    ) {
      throw new Error(
        "requestId, blob, and version are required for storeBlob",
      );
    }
    this.validateLambdaExecuteId(args["requestId"]);
    const blob = args["blob"];
    if (!(blob instanceof Blob)) {
      throw new Error(
        "store() expects a Blob. If you are trying to store a Request, `await request.blob()` will give you the correct input.",
      );
    }

    const headers: Record<string, string> = { "Content-Type": blob.type };
    const options = args["options"];
    if (options?.sha256 !== undefined) {
      headers["Digest"] = `sha-256=${options.sha256}`;
    }

    const uploadUrl = await this._storageGenerateUploadUrl(args["version"]);
    const response = await fetch(uploadUrl, {
      method: "POST",
      body: blob,
      headers: headers,
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Error uploading file: ${text}`);
    }
    const respJSON = await response.json();
    if (respJSON.storageId === undefined) {
      throw new Error("Did not get a storageId in store blob response");
    }
    return respJSON.storageId;
  }

  async syscallGetBlob(args: Record<string, any>): Promise<any> {
    if (
      args["requestId"] === undefined ||
      args["storageId"] === undefined ||
      args["version"] === undefined
    ) {
      throw new Error(
        "requestId, storageId, and version are required for getBlob",
      );
    }
    this.validateLambdaExecuteId(args["requestId"]);

    const getUrl = await this._storageGetUrl({
      storageId: args["storageId"],
      version: args["version"],
    });
    if (getUrl === null) {
      return null;
    }
    const getResult = await fetch(getUrl);
    return await getResult.blob();
  }

  async syscallCreateFunctionHandle(rawArgs: string): Promise<JSONValue> {
    const createFunctionHandleArgs = z.object({
      name: z.optional(z.string()),
      reference: z.optional(z.string()),
      version: z.string(),
    });
    const operationName = "create function handle";
    const args = this.validateArgs(
      rawArgs,
      createFunctionHandleArgs,
      operationName,
      false,
    );
    const { handle } = await this.actionCallback({
      version: args.version,
      body: {
        udfPath: args.name,
        reference: args.reference,
      },
      path: "/api/actions/create_function_handle",
      operationName,
      responseValidator: z.any(),
      retryTransient: false,
    });
    return handle;
  }
}

function forwardErrorData(errorData: JSONValue, error: ConvexError<string>) {
  (error as ConvexError<any>).data = errorData;
  return error;
}

function stripExtension(path: string): string {
  return path.replace(/\.[^/.]+$/, "");
}
