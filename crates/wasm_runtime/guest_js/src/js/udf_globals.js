const unwrap = (raw) => {
  const envelope = JSON.parse(raw);
  if (!envelope.ok) {
    throw new Error(envelope.message);
  }
  return envelope.value;
};

globalThis.Convex = {
  syscall: (op, argsJson) => {
    globalThis.__convex_require_host_call("Convex.syscall");
    return unwrap(__convex_syscall(op, argsJson));
  },
  asyncSyscall: (op, argsJson) => {
    globalThis.__convex_require_host_call("Convex.asyncSyscall");
    return new Promise((resolve, reject) => {
      globalThis.__convex_track_op(
        __convex_start_async_syscall(op, argsJson),
        resolve,
        reject,
      );
    });
  },
  op: (opName) => {
    throw new Error(
      `The op \`${opName}\` is not implemented in the wasm runtime yet`,
    );
  },
  jsSyscall: (op) => {
    throw new Error(
      `The JS syscall \`${op}\` is not implemented in the wasm runtime yet`,
    );
  },
};

// A handful of globals the bundled `convex` package reaches for are backed by
// the V8 ops layer. Until that layer is ported, the few that matter are served
// as reserved sync syscalls under an `op/` prefix.
const hostOp = (name, args) => {
  globalThis.__convex_require_host_call(name);
  return JSON.parse(unwrap(__convex_syscall(name, JSON.stringify(args))));
};

globalThis.process = {
  env: new Proxy(
    {},
    {
      get: (target, prop, receiver) => {
        if (typeof prop !== "string") {
          return Reflect.get(target, prop, receiver);
        }
        const value = hostOp("op/environmentVariables/get", [prop]);
        // Map null to undefined: libraries check for undefined, and the host
        // has no undefined to send.
        return value === null ? undefined : value;
      },
    },
  ),
};

// QuickJS's own `Math.random` would make functions nondeterministic, so it is
// replaced with the host's seeded RNG the same way `setup.ts` does.
delete globalThis.Math.random;
globalThis.Math.random = () => hostOp("op/random", []);

globalThis.self = globalThis;

// The V8 runtime formats console arguments with the `object-inspect` package,
// which quotes strings and has its own layout for objects. This matches it for
// scalars — the common case, and the one log assertions are usually about — but
// composite values still come out as JSON, so they read differently.
const formatLogArg = (value) => {
  if (typeof value === "string") {
    return `'${JSON.stringify(value).slice(1, -1)}'`;
  }

  if (value === undefined || value === null || typeof value !== "object") {
    return String(value);
  }

  try {
    return JSON.stringify(value);
  } catch (_error) {
    return String(value);
  }
};

const logAt =
  (level) =>
  (...args) => {
    globalThis.__convex_require_host_call("console." + level);
    __convex_syscall(
      "console/message",
      JSON.stringify({ level, messages: args.map(formatLogArg) }),
    );
  };

globalThis.console = {
  log: logAt("log"),
  info: logAt("info"),
  warn: logAt("warn"),
  error: logAt("error"),
  debug: logAt("debug"),
};
