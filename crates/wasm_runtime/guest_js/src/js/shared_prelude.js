globalThis.__convex_host_calls_enabled = false;
globalThis.__convex_pending_ops = new Map();
globalThis.__convex_active_invocation = null;

globalThis.__convex_require_host_call = (name) => {
  if (!globalThis.__convex_host_calls_enabled) {
    throw new Error(name + " is unavailable during module initialization");
  }
};

globalThis.__convex_track_op = (opId, resolve, reject) => {
  globalThis.__convex_pending_ops.set(opId, { resolve, reject });
};
