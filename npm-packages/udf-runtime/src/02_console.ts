import inspect from "object-inspect";
import { performOp } from "udf-syscall-ffi";

function getMessage(args) {
  // TODO: Support string substitution.
  const serializedArgs = args.map((e) =>
    inspect(e, {
      // Our entire log line can't be more than 32768 bytes (MAX_LOG_LINE_LENGTH) so
      // keep string in here to no more than 32768 UTF-16 code units, and let
      // the backend truncate the whole log line if it is too long.
      maxStringLength: 32768,
      indent: 2,
      customInspect: true,
    }),
  );
  return serializedArgs;
}

function toString(value: unknown, defaultValue: string) {
  return value === undefined
    ? defaultValue
    : value === null
      ? "null"
      : value.toString();
}

function consoleMessage(level, args) {
  performOp("console/message", level, getMessage(args));
}
const consoleImpl = {
  debug: function (...args) {
    consoleMessage("DEBUG", args);
  },
  error: function (...args) {
    consoleMessage("ERROR", args);
  },
  info: function (...args) {
    consoleMessage("INFO", args);
  },
  log: function (...args) {
    consoleMessage("LOG", args);
  },
  warn: function (...args) {
    consoleMessage("WARN", args);
  },
  trace: function (...args) {
    const message = getMessage(args);
    const error = new Error();
    // This calls `prepareStackTrace` that populates `__frameData`
    error.stack;
    const frameData = JSON.parse((error as any).__frameData ?? []);
    performOp("console/trace", message, frameData);
  },
  time: function (label: unknown) {
    const labelStr = toString(label, "default");
    performOp("console/timeStart", labelStr);
  },
  timeLog: function (label: unknown, ...args) {
    const labelStr = toString(label, "default");
    performOp("console/timeLog", labelStr, getMessage(args));
  },
  timeEnd: function (label: unknown) {
    const labelStr = toString(label, "default");
    performOp("console/timeEnd", labelStr);
  },
  // TODO: Implement the rest of the Console API.
};
export function setupConsole(global) {
  // Delete v8's console since it doesn't go anywhere. We'll eventually want to mirror our console
  // output to v8's console since apparently its output shows up in v8's debugger.
  delete global.console;
  global.console = consoleImpl;
}
