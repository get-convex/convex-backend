const awsConsole = console;
let globalStdoutLogEnabled = false;

export function setDebugLogging(enabled: boolean) {
  globalStdoutLogEnabled = enabled;
}

// Logs and returns the elapsed time in milliseconds.
export function logDurationMs(label: string, start: number): number {
  const elapsed = performance.now() - start;
  if (globalStdoutLogEnabled) {
    awsConsole.log(`${label}: ${elapsed.toFixed(2)}ms`);
  }
  return elapsed;
}

export function logDebug(message: string) {
  if (globalStdoutLogEnabled) {
    awsConsole.log(message);
  }
}

export function log(message: string) {
  awsConsole.log(message);
}
