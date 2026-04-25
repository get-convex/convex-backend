import { performSyscall } from "./impl/syscall.js";

const REQUEST_ID = Symbol("var.requestId");
const IP = Symbol("var.ip");
const USER_AGENT = Symbol("var.userAgent");
const NOW = Symbol("var.now");

type AuditVar = typeof REQUEST_ID | typeof IP | typeof USER_AGENT | typeof NOW;

const varNames: Record<symbol, string> = {
  [REQUEST_ID]: "requestId",
  [IP]: "ip",
  [USER_AGENT]: "userAgent",
  [NOW]: "now",
};

export type AuditLogBody = { [key: string]: AuditLogValue };
export type AuditLogValue =
  | null
  | undefined
  | boolean
  | number
  | string
  | AuditVar
  | AuditLogValue[]
  | { [key: string]: AuditLogValue };

type JsonValue =
  | null
  | undefined
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

function validateKey(key: string) {
  if (key.startsWith("$")) {
    throw new Error(`Audit log body keys must not start with "$": "${key}"`);
  }
}

function cloneValue(value: AuditLogValue): JsonValue {
  if (typeof value === "symbol") {
    if (!(value in varNames)) {
      throw new Error(
        `Unknown audit var symbol: ${String(value)}. Use one of audit.var.requestId, audit.var.ip, audit.var.userAgent, or audit.var.now.`,
      );
    }
    return { $var: varNames[value] };
  }
  if (value === null || value === undefined || typeof value !== "object") {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map(cloneValue);
  }
  const result: { [key: string]: JsonValue } = {};
  for (const [key, val] of Object.entries(value)) {
    validateKey(key);
    result[key] = cloneValue(val);
  }
  return result;
}

/**
 * Deep-clone the body, replacing audit var symbols with sentinel objects
 * like `{ $var: "ip" }`.
 */
export function cloneWithSentinels(body: AuditLogBody): {
  [key: string]: JsonValue;
} {
  const result: { [key: string]: JsonValue } = {};
  for (const [key, val] of Object.entries(body)) {
    validateKey(key);
    result[key] = cloneValue(val);
  }
  return result;
}

const auditVars = {
  /** Resolved to the request ID. */
  requestId: REQUEST_ID,
  /** Resolved to the client's IP address. */
  ip: IP,
  /** Resolved to the client's User-Agent header. */
  userAgent: USER_AGENT,
  /**
   * Resolved to the current server timestamp, as milliseconds from the
   * Unix epoch.
   */
  now: NOW,
} as const;

/**
 * Audit logging API. Use `audit.log()` to emit an audit log line from a
 * Convex function. The body can contain `audit.var` sentinels that are
 * resolved server-side when the log line is recorded.
 *
 * @internal
 */
export const audit = {
  log: (body: AuditLogBody) => {
    performSyscall("1.0/auditLog", { body: cloneWithSentinels(body) });
  },

  var: auditVars,
};
