globalThis.__convex_start_udf_invoke = (functionName, udfType, argsJson) => {
  const settle = (payload) => {
    globalThis.__convex_active_invocation = {
      settled: true,
      result: JSON.stringify(payload),
    };
  };

  if (globalThis.__convex_active_invocation !== null) {
    throw new Error("invoke already in flight for this instance");
  }

  const fn = globalThis.__convex_exports?.[functionName];
  if (fn === undefined || fn === null) {
    settle({ ok: false, error: { kind: "FunctionNotFound", functionName } });
    return;
  }

  const isQuery = fn.isQuery === true;
  const isMutation = fn.isMutation === true;
  const invoke =
    udfType === "query" && isQuery && !isMutation
      ? fn.invokeQuery
      : udfType === "mutation" && isMutation && !isQuery
        ? fn.invokeMutation
        : undefined;

  if (typeof invoke !== "function") {
    settle({
      ok: false,
      error: { kind: "FunctionType", udfType, isQuery, isMutation },
    });
    return;
  }

  const invocation = { settled: false, result: null };
  globalThis.__convex_active_invocation = invocation;
  globalThis.__convex_host_calls_enabled = true;

  Promise.resolve()
    .then(() => invoke(argsJson))
    .then(
      (value) => {
        invocation.settled = true;
        invocation.result = JSON.stringify({ ok: true, value });
      },
      (error) => {
        invocation.settled = true;
        invocation.result = JSON.stringify({
          ok: false,
          error: {
            kind: "HandlerError",
            message: error instanceof Error ? error.message : String(error),
            stack: error instanceof Error ? (error.stack ?? null) : null,
            data:
              error instanceof Error && error.data !== undefined
                ? error.data
                : null,
          },
        });
      },
    )
    .finally(() => {
      globalThis.__convex_host_calls_enabled = false;
    });
};
