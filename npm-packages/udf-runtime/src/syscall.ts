// This object gets set up in Rust by `setup_context`.
declare const Convex: {
  op: (op: string, ...args: any[]) => any;
  asyncOp: (op: string, ...args: any[]) => Promise<any>;
  syscall: (op: string, jsonArgs: string) => string;
  asyncSyscall: (op: string, jsonArgs: string) => Promise<string>;
  jsSyscall: (op: string, args: Record<string, any>) => any;
};

/**
 * Perform an "op" -- this is similar to `performSyscall` in many ways (it takes in
 * and returns JSON, and calls into Rust). However unlike syscalls, ops do not
 * need to be backwards compatible with `convex/server` since they are only used
 * within `udf-runtime` which is pushed with the Rust implementations.
 *
 * +------------+------------------------+-------------------------+
 * |            | convex/server          | udf-runtime             |
 * +------------+------------------------+-------------------------+
 * | JS to Rust | perform(Async)?Syscall | performOp               |
 * +------------+------------------------+-------------------------+
 * | JS to JS   | performJsSyscall       | call functions directly |
 * +------------+------------------------+-------------------------+
 *
 * Code in `convex/server` is versioned with the NPM package and pushed by developers
 * when they deploy convex functions, while the implementations for the syscalls
 * are versioned with `backend`. This means changes to syscalls must be backwards
 * compatible with older NPM versions (i.e. it is not safe to rename a syscall).
 *
 * In contrast, code in `udf-runtime` is pushed with `backend`, so it is safe to
 * change an `op` without worrying about backwards compatibility, since ops are
 * only used in `udf-runtime`.
 *
 * @param op
 * @param arg
 * @returns
 */
export function performOp(op: string, ...args: any[]): any {
  if (typeof Convex === "undefined" || Convex.op === undefined) {
    throw new Error(
      "The Convex execution environment is being unexpectedly run outside of a Convex backend.",
    );
  }
  return Convex.op(op, ...args);
}

export async function performAsyncOp(op: string, ...args: any[]): Promise<any> {
  if (typeof Convex === "undefined" || Convex.asyncOp === undefined) {
    throw new Error(
      "The Convex execution environment is being unexpectedly run outside of a Convex backend.",
    );
  }
  return await Convex.asyncOp(op, ...args);
}

/**
 * Perform a syscall, taking in a JSON-encodable object as an argument, serializing with
 * JSON.stringify, calling into Rust, and then parsing the response as a JSON-encodable
 * value. If one of your arguments is a Convex value, you must call `convexToJson` on it
 * before passing it to this function, and if the return value has a Convex value, you're
 * also responsible for calling `jsonToConvex`: This layer only deals in JSON.
 */
export function performSyscall(op: string, arg: Record<string, any>): any {
  if (typeof Convex === "undefined" || Convex.syscall === undefined) {
    throw new Error(
      "The Convex database and auth objects are being used outside of a Convex backend. " +
        "Did you mean to use `useQuery` or `useMutation` to call a Convex function?",
    );
  }
  const resultStr = Convex.syscall(op, JSON.stringify(arg));
  return JSON.parse(resultStr);
}

export async function performAsyncSyscall(
  op: string,
  arg: Record<string, any>,
): Promise<any> {
  if (typeof Convex === "undefined" || Convex.asyncSyscall === undefined) {
    throw new Error(
      "The Convex database and auth objects are being used outside of a Convex backend. " +
        "Did you mean to use `useQuery` or `useMutation` to call a Convex function?",
    );
  }
  let resultStr;
  try {
    resultStr = await Convex.asyncSyscall(op, JSON.stringify(arg));
  } catch (e: any) {
    // Rethrow the exception to attach stack trace starting from here.
    // If the error came from JS it will include its own stack trace.
    // If it came from Rust it won't.
    throw new Error(e.message);
  }
  return JSON.parse(resultStr);
}
