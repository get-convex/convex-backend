const requireHostCall = globalThis.__convex_require_host_call;

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

const unwrap = (raw) => {
  const envelope = JSON.parse(raw);
  if (!envelope.ok) {
    throw new Error(envelope.message);
  }
  return envelope.value;
};

const syscall = (name, args) =>
  JSON.parse(unwrap(__convex_syscall(name, JSON.stringify(args))));

const logAt =
  (level) =>
  (...args) => {
    requireHostCall("console." + level);
    __convex_syscall(
      "console/message",
      JSON.stringify({
        level,
        messages: [args.map(formatConsoleArg).join(" ")],
      }),
    );
  };

// The fixtures' `db` is a sync syscall behind an already-resolved promise, so
// it exercises the microtask queue without needing the host's async op table.
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
    log: logAt("log"),
    warn: logAt("warn"),
    error: logAt("error"),
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
};
