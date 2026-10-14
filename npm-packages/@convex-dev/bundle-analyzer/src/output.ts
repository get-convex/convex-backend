/** A failure the CLI reports as a message and exit code 1, without a stack. */
export class AnalyzerError extends Error {}

/** Writes a report or JSON to stdout. Progress and warnings go to stderr. */
export function logOutput(text: string) {
  process.stdout.write(text + "\n");
}

export function logWarning(text: string) {
  process.stderr.write(text + "\n");
}
