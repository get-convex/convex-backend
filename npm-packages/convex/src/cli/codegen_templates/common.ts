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

/**
 * We generate JS files as .d.ts and .js so that they are usable by both
 * JavaScript and TypeScript developers.
 */
export type GeneratedJsWithTypes = {
  DTS: string;
  JS?: string;
};
