type CaptureSeverity = "fatal" | "error" | "warning" | "log" | "debug" | "info";

const emptyDataPageWarning =
  "Encountered unexpected state in data page: status: Exhausted, numRowsInTable: 0, numRowsRead: 0, isLoading: false";
const transientFunctionTreeLoadError =
  "File tree map called before modules or nents were loaded";

export function shouldForwardDeploymentCaptureMessage(
  message: string,
  severity: CaptureSeverity,
) {
  if (severity === "warning" && message === emptyDataPageWarning) {
    return false;
  }
  if (severity === "error" && message === transientFunctionTreeLoadError) {
    return false;
  }
  return true;
}
