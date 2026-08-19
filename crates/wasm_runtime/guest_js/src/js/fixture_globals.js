const requireHostCall = globalThis.__convex_require_host_call;
const trackOp = globalThis.__convex_track_op;

const formatConsoleArg = (value) => {
  if (typeof value === "string") {
    return value;
  }

  try {
    return JSON.stringify(value);
  } catch (_error) {
    return String(value);
  }
};

const normalizeHeaders = (headers) => {
  if (!headers) {
    return [];
  }

  if (Array.isArray(headers)) {
    return headers.map(([name, value]) => [String(name), String(value)]);
  }

  return Object.entries(headers).map(([name, value]) => [
    String(name),
    String(value),
  ]);
};

const syscall = (name, args) =>
  JSON.parse(__convex_syscall(name, JSON.stringify(args)));

const startAsyncSyscall = (name, args) =>
  __convex_start_async_syscall(name, JSON.stringify(args));

const makeResponse = (response) => ({
  status: response.status,
  ok: response.ok,
  url: response.url,
  headers: Object.fromEntries(response.headers ?? []),
  text: async () => response.body_text ?? "",
  json: async () => JSON.parse(response.body_text ?? "null"),
});

globalThis.__convex_runtime = {
  db: {
    get: (key) => {
      requireHostCall("db.get");
      return Promise.resolve().then(() => syscall("db/get", [String(key)]));
    },
    set: (key, value) => {
      requireHostCall("db.set");
      return Promise.resolve().then(() => {
        syscall("db/set", [String(key), value]);
        return value;
      });
    },
    delete: (key) => {
      requireHostCall("db.delete");
      return Promise.resolve().then(() => syscall("db/delete", [String(key)]));
    },
  },
  console: {
    log: (...args) => {
      requireHostCall("console.log");
      syscall("console/message", [args.map(formatConsoleArg).join(" ")]);
    },
    warn: (...args) => {
      requireHostCall("console.warn");
      syscall("console/message", [args.map(formatConsoleArg).join(" ")]);
    },
    error: (...args) => {
      requireHostCall("console.error");
      syscall("console/message", [args.map(formatConsoleArg).join(" ")]);
    },
  },
  crypto: {
    randomUUID: () => {
      requireHostCall("crypto.randomUUID");
      return syscall("crypto/randomUuid", []);
    },
  },
  now: () => {
    requireHostCall("now");
    return syscall("time/now", []);
  },
  sleep: (ms) => {
    requireHostCall("sleep");
    return new Promise((resolve, reject) => {
      trackOp(startAsyncSyscall("sleep", [Number(ms) | 0]), resolve, reject);
    });
  },
};

globalThis.fetch = (input, init = {}) => {
  requireHostCall("fetch");

  const request = {
    url: String(input),
    method: String(init.method ?? "GET").toUpperCase(),
    headers: normalizeHeaders(init.headers),
    body: init.body == null ? null : String(init.body),
  };

  return new Promise((resolve, reject) => {
    trackOp(
      startAsyncSyscall("fetch", [request]),
      (payload) => resolve(makeResponse(payload)),
      reject,
    );
  });
};
