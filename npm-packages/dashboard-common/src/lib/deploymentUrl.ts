export function deploymentUrlForBrowser(
  deploymentUrl: string,
  pageProtocol = currentPageProtocol(),
): string {
  if (pageProtocol !== "https:") {
    return deploymentUrl;
  }
  try {
    const url = new URL(deploymentUrl);
    if (url.protocol === "http:") {
      url.protocol = "https:";
      return url.toString().replace(/\/$/, "");
    }
  } catch {
    return deploymentUrl;
  }
  return deploymentUrl;
}

function currentPageProtocol(): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }
  return window.location.protocol;
}
