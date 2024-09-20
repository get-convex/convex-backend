export const toReferencePath = Symbol.for("toReferencePath");

// Multiple instances of the same Symbol.for() are equal at runtime but not
// at type-time, so `[toReferencePath]` properties aren't used in types.
// Use this function to set the property invisibly.
export function setReferencePath<T>(obj: T, value: string) {
  (obj as any)[toReferencePath] = value;
}

export function extractReferencePath(reference: any): string | null {
  return reference[toReferencePath] ?? null;
}

export function isFunctionHandle(s: string): boolean {
  return s.startsWith("function://");
}
