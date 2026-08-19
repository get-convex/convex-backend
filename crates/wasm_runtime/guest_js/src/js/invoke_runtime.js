if (typeof __convex_exports !== "undefined") {
  globalThis.__convex_exports = __convex_exports;
}

globalThis.__convex_poll_pending_ops = () => {
  for (const opId of JSON.parse(__convex_completed_ops())) {
    const entry = globalThis.__convex_pending_ops.get(opId);
    if (entry === undefined) {
      continue;
    }

    const result = JSON.parse(__convex_take_op_result(opId));
    globalThis.__convex_pending_ops.delete(opId);

    if (result.ok) {
      entry.resolve(result.value);
    } else {
      entry.reject(new Error(result.message));
    }
  }

  return globalThis.__convex_pending_ops.size;
};

globalThis.__convex_pending_op_count = () =>
  globalThis.__convex_pending_ops.size;

globalThis.__convex_start_invoke = (handlerName, argsJson) => {
  const respond = (payload) => JSON.stringify(payload);
  const handlers = globalThis.__convex_exports;
  const handler = handlers?.[handlerName];

  if (globalThis.__convex_active_invocation !== null) {
    throw new Error("invoke already in flight for this instance");
  }

  if (typeof handler !== "function") {
    globalThis.__convex_active_invocation = {
      settled: true,
      result: respond({
        ok: false,
        error: {
          kind: "MissingHandler",
          message: `handler ${JSON.stringify(handlerName)} was not found`,
        },
      }),
    };
    return;
  }

  let args;
  try {
    args = JSON.parse(argsJson);
  } catch (error) {
    globalThis.__convex_active_invocation = {
      settled: true,
      result: respond({
        ok: false,
        error: {
          kind: "InvalidArgs",
          message: error instanceof Error ? error.message : String(error),
        },
      }),
    };
    return;
  }

  if (!Array.isArray(args)) {
    globalThis.__convex_active_invocation = {
      settled: true,
      result: respond({
        ok: false,
        error: {
          kind: "InvalidArgs",
          message: "invoke args must decode to a JSON array",
        },
      }),
    };
    return;
  }

  const invocation = {
    settled: false,
    result: null,
  };

  globalThis.__convex_active_invocation = invocation;
  globalThis.__convex_host_calls_enabled = true;

  Promise.resolve()
    .then(() => handler(...args))
    .then(
      (value) => {
        invocation.settled = true;
        invocation.result = respond({ ok: true, value });
      },
      (error) => {
        invocation.settled = true;
        invocation.result = respond({
          ok: false,
          error: {
            kind: "HandlerError",
            message: error instanceof Error ? error.message : String(error),
            stack: error instanceof Error ? (error.stack ?? null) : null,
          },
        });
      },
    )
    .finally(() => {
      globalThis.__convex_host_calls_enabled = false;
    });
};

globalThis.__convex_take_invoke_result = () => {
  const invocation = globalThis.__convex_active_invocation;
  if (!invocation || !invocation.settled) {
    return null;
  }

  const result = invocation.result;
  globalThis.__convex_active_invocation = null;
  return result;
};
