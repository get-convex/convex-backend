export function joinUrlPath(baseUrl: string, path: string): URL {
  // `new URL("/api/...", base)` discards any path prefix in `base`.
  // Also note: `new URL("./def", "https://a.com/abc")` => `/def` because `abc`
  // is treated like a file; ensure the base pathname ends with `/`.
  const base = new URL(baseUrl);
  if (!base.pathname.endsWith("/")) {
    base.pathname = `${base.pathname}/`;
  }
  const relativePath = path.replace(/^\//, "");
  return new URL(relativePath, base);
}
