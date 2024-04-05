export interface FrameData {
  typeName: string | null;
  functionName: string | null;
  methodName: string | null;
  fileName: string | null;
  lineNumber: number | null;
  columnNumber: number | null;
  evalOrigin: string | null;
  isToplevel: boolean | null;
  isEval: boolean;
  isNative: boolean;
  isConstructor: boolean;
  isAsync: boolean;
  isPromiseAll: boolean;
  promiseIndex: number | null;
}

// https://v8.dev/docs/stack-trace-api#appendix%3A-stack-trace-format
function formatTraceLine(frame: FrameData) {
  let displayFile = frame.fileName;

  // strip query params used for cachebusting environment
  displayFile = (displayFile || "").replace(/\.js\?.*/, ".js");

  // if it doesn't start with convex:/ or node:/ then it might be
  // bundled file or it might be external.
  // TODO deal with external dependencies (I think node_modules/*)
  if (!displayFile) {
    displayFile = "";
  } else if (displayFile.startsWith("convex:")) {
    // leave it alone
  } else if (displayFile.startsWith("node:")) {
    // leave it alone
  } else {
    displayFile = "bundledFunctions.js";
  }

  const location = frame.fileName
    ? `${displayFile}:${frame.lineNumber}:${frame.columnNumber}`
    : "<unknown location>";

  // TODO [as methodName]

  const func = frame.functionName || frame.methodName || "";

  if (func) {
    return `    at${frame.isAsync ? " async" : ""} ${func} (${location})`;
  } else {
    // When code doesn't have a name called show only the location.
    return `    at ${location}`;
  }
}

export function registerPrepareStackTrace(modulesDir: string) {
  // This function is called on-demand when the `stack` property of an `Error` is accessed for the first time.
  // See https://v8.dev/docs/stack-trace-api for more details.
  Error.prepareStackTrace = (error, stackFrames) => {
    const frameData: FrameData[] = stackFrames.map((v8Frame) => {
      let fileName = v8Frame.getFileName();
      // For source mapping to work, all user modules need to start with
      // "convex:/user". Replace the full path with that. Note that we use
      // indexOf() instead of startsWith() since there might be different
      // prefixes like "file://" or "file://private/" and we don't want to
      // enumerate them all since those might differ between mac and linux.
      if (fileName) {
        const index = fileName.indexOf(modulesDir);
        if (index !== -1) {
          fileName =
            "convex:/user" + fileName.substring(index + modulesDir.length);
        }
      }
      return {
        typeName: v8Frame.getTypeName(),
        functionName: v8Frame.getFunctionName(),
        methodName: v8Frame.getMethodName(),
        fileName: fileName ?? null,
        lineNumber: v8Frame.getLineNumber(),
        columnNumber: v8Frame.getColumnNumber(),
        evalOrigin: v8Frame.getEvalOrigin() ?? null,
        isToplevel: v8Frame.isToplevel(),
        isEval: v8Frame.isEval(),
        isNative: v8Frame.isNative(),
        isConstructor: v8Frame.isConstructor(),
        isAsync: (v8Frame as any).isAsync() as boolean,
        isPromiseAll: (v8Frame as any).isPromiseAll() as boolean,
        promiseIndex: (v8Frame as any).getPromiseIndex() as number | null,
      };
    });
    // We currently always go through JSON when going over the JS <-> Rust boundary. Eventually we can make this more efficient by accessing the V8 objects directly in Rust.
    const frameJSON = JSON.stringify(frameData);
    // Save the structured frame data on the exception so we can use it from Rust later.
    Object.defineProperties(error, {
      __frameData: { value: frameJSON, configurable: true },
    });
    // For now, we don't expose the source mapped stack to userspace: The only way to get a good traceback is to throw an exception and have the Rust layer catch it.
    // After evaluating a UDF and catching its error, the Rust layer loads the source map and does its best to get a good traceback.
    //
    // Some libraries like https://github.com/TooTallNate/proxy-agents/blob/c169ced054272e30d619746c0d0673d0b8337e06/packages/agent-base/src/index.ts#L8-L18 rely
    // on Node.js-formatted stack traces to work. This doesn't require anything be mapped to original sources.
    //
    // TODO find a library to do this properly: once we provide it libraries will depend on it matching Node.js stack traces.

    return `Error\n${frameData
      .map((frame) => formatTraceLine(frame))
      .join("\n")}`;
  };
}

// Extract an error message from an exception throw by untrusted source.
export function extractErrorMessage(e: any): string {
  if (e === null || e === undefined) {
    return "unknown error";
  }

  try {
    if (typeof e.message?.toString === "function") {
      const errorMessage = e.message.toString();
      // Make sure toString() returns a string.
      if (typeof errorMessage === "string") {
        return errorMessage;
      }
    } else if (typeof e.toString === "function") {
      const errorMessage = e.toString();
      // Make sure toString() returns a string.
      if (typeof errorMessage === "string") {
        return errorMessage;
      }
    }
    return "unknown error";
  } catch {
    // toString threw an error?!
    return "unknown error";
  }
}
