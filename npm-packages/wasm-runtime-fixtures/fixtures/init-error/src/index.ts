if (!globalThis.__FELIX_BUNDLE_INTROSPECTION__) {
  throw new Error("top-level init exploded");
}

export function neverRuns() {
  return "unreachable";
}
