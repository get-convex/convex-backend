export const toReferencePath = Symbol.for("toReferencePath");

export function extractReferencePath(reference: any): string | null {
  return reference[toReferencePath] ?? null;
}
