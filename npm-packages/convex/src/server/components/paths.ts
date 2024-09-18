export const toReferencePath = Symbol.for("toReferencePath");

export function extractReferencePath(reference: any): string | null {
  return reference[toReferencePath] ?? null;
}

export function isFunctionHandle(s: string): boolean {
  return s.startsWith("function://");
}
