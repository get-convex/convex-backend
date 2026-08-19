globalThis.__convexInitCount = (globalThis.__convexInitCount ?? 0) + 1;

export function initCount() {
  return globalThis.__convexInitCount;
}

export function describeInit() {
  return {
    initCount: globalThis.__convexInitCount,
    status: globalThis.__convexInitCount === 1 ? "preinitialized" : "reran",
  };
}
