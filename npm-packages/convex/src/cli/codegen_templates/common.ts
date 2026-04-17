export function header(oneLineDescription: string) {
  return `/* eslint-disable */
  /**
   * ${oneLineDescription}
   *
   * THIS CODE IS AUTOMATICALLY GENERATED.
   *
   * To regenerate, run \`npx convex dev\`.
   * @module
   */
  `;
}

export function apiComment(
  apiName: string,
  type: "public" | "internal" | undefined,
) {
  return `
  /**
     * A utility for referencing Convex functions in your app's${type ? ` ${type}` : ""} API.
     *
     * Usage:
     * \`\`\`js
     * const myFunctionReference = ${apiName}.myModule.myFunction;
     * \`\`\`
     */`;
}

const collator = new Intl.Collator("en-US", {
  usage: "sort",
  numeric: true,
  sensitivity: "case",
  ignorePunctuation: false,
  caseFirst: "false",
});

/**
 * Comparison function for sorting strings alphabetically.
 *
 * Usage: array.sort(compareStrings)
 * or with entries: Object.entries(obj).sort(([a], [b]) => compareStrings(a, b))
 */
export function compareStrings(a: string, b: string): number {
  return collator.compare(a, b);
}
