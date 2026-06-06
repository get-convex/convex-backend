type CaptureSeverity = "fatal" | "error" | "warning" | "log" | "debug" | "info";

const emptyDataPageWarning =
  "Encountered unexpected state in data page: status: Exhausted, numRowsInTable: 0, numRowsRead: 0, isLoading: false";

export function shouldForwardDeploymentCaptureMessage(
  message: string,
  severity: CaptureSeverity,
) {
  return severity !== "warning" || message !== emptyDataPageWarning;
}
