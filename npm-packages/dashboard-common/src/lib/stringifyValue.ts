import { Value } from "convex/values";
import * as Base64 from "base64-js";
// @ts-expect-error
import isValidIdentifier from "is-valid-identifier";
import { prettier } from "lib/format";

function stringify(value: Value): string {
  // TODO: Remove this branch when we have more type-safety.
  if (typeof value === "undefined") {
    return "undefined";
  }
  if (value === null) {
    return "null";
  }
  if (typeof value === "bigint") {
    return `${value.toString()}n`;
  }
  if (typeof value === "number") {
    return value.toString();
  }
  if (typeof value === "boolean") {
    return value.toString();
  }
  if (typeof value === "string") {
    return value.includes("\n")
      ? // Multiline strings should be rendered using backticks
        `\`${value.replace(/(?<!\\)`/g, "\\`").replace(/\$\{/g, "\\${")}\``
      : // Use `JSON.stringify`'s string escaping.
        JSON.stringify(value);
  }
  if (value instanceof ArrayBuffer) {
    const base64Encoded = Base64.fromByteArray(new Uint8Array(value));
    return `Bytes("${base64Encoded}")`;
  }
  if (value instanceof Array) {
    return `[${value.map(stringify).join(", ")}]`;
  }
  // Assume we have an object at this point.
  const pairs = Object.entries(value)
    .map(
      ([k, v]) =>
        `${isValidIdentifier(k) ? k : stringify(k)}: ${stringify(v!)}`,
    )
    .join(", ");
  return `{ ${pairs} }`;
}

export function stringifyValue(
  value: Value,
  multiline: boolean = false,
  forceMultiline: boolean = false,
): string {
  let oneline = stringify(value);
  if (!multiline) {
    return oneline;
  }
  // Turn our JS expression into a statement, feed it into prettier, and then get the expression back.
  const header = `const value = `;
  const footer = ";";
  if (forceMultiline) {
    if (oneline.startsWith("{")) {
      oneline = `{\n${oneline.slice(1)}`;
    } else if (oneline.startsWith("[{")) {
      oneline = `[{\n${oneline.slice(2)}`;
    }
  }
  const stmt = `${header}${oneline}${footer}`;
  let prettierExpr = prettier(stmt);
  if (prettierExpr.startsWith(header.trimEnd())) {
    prettierExpr = prettierExpr.slice(header.length);
  }
  if (prettierExpr.endsWith(footer)) {
    prettierExpr = prettierExpr.slice(0, -footer.length);
  }
  return prettierExpr.trimStart();
}
