import { Identifier, LineNumber } from "./convex";

// Match the original declaration of a variable.
const definitionRegex = /^var ([a-zA-Z0-9_]+) =/;

// Match a one line export.
const onelineExportRegex = /^export \{([a-zA-Z0-9_, ]+)\};$/;
const onelineItemRegex = /^([a-zA-Z0-9_]+)( as [a-zA-Z0-9_]+)?$/;

// Match a line in the `export` block at the end of an esbuild bundle.
const exportRegex = /^([a-zA-Z0-9_]+)( as [a-zA-Z0-9_]+)?,?$/;

export function findLineNumbers(
  source: string,
  expectedExports: Identifier[],
): Map<Identifier, LineNumber> {
  const lines = source.split("\n").map((l) => l.trim());

  // First, find the `export` block at the end of the bundle. For example,
  // ```
  // export {
  //   a_default as default,
  //   b,
  // };
  // ```
  /// We also have to handle the case where esbuild decides to emit a oneline export.
  // ```
  // export { a as b, b };
  let foundExport = false;
  const exportMap = new Map<string, string>();
  for (const line of lines) {
    const onelineMatch = line.match(onelineExportRegex);
    if (onelineMatch) {
      const pieces = onelineMatch[1].split(",").map((l) => l.trim());
      for (const piece of pieces) {
        const itemMatch = piece.match(onelineItemRegex);
        if (!itemMatch) {
          console.warn(`Item ${piece} in ${line} didn't match item regex`);
          continue;
        }
        const origIdentifier = itemMatch[1];
        const exportIdentifier = itemMatch[2]
          ? itemMatch[2].slice(" as ".length)
          : origIdentifier;
        exportMap.set(exportIdentifier, origIdentifier);
      }
      break;
    }
    if (line === "export {") {
      foundExport = true;
      continue;
    }
    if (foundExport && line === "};") {
      foundExport = false;
      break;
    }
    if (!foundExport) {
      continue;
    }
    const exportMatch = line.match(exportRegex);
    if (!exportMatch) {
      console.warn(`Line ${line} did not match export regex`);
      continue;
    }
    const origIdentifier = exportMatch[1];
    const exportIdentifier = exportMatch[2]
      ? exportMatch[2].slice(" as ".length)
      : origIdentifier;
    exportMap.set(exportIdentifier, origIdentifier);
  }
  // Do a second pass where we try to find `esbuild`'s original declarations.
  // ```
  // var a_default = action(() => { ... });
  // ```
  const lineNumbers = new Map<string, number>();
  for (const [index, line] of lines.entries()) {
    const lineno = index + 1;
    const definitionMatch = line.match(definitionRegex);
    if (!definitionMatch) {
      continue;
    }
    const origIdentifier = definitionMatch[1];
    lineNumbers.set(origIdentifier, lineno);
  }

  // Stitch the two relations together to map exported name to line number.
  const result = new Map<string, number>();
  for (const exported of expectedExports) {
    const origIdentifier = exportMap.get(exported);
    if (origIdentifier === undefined) {
      console.warn(`Couldn't find export ${exported} in `, exportMap);
      continue;
    }
    const lineNumber = lineNumbers.get(origIdentifier);
    if (lineNumber === undefined) {
      console.warn(
        `Couldn't find line number for ${exported} -> ${origIdentifier} in `,
        lineNumbers,
      );
      continue;
    }
    result.set(exported, lineNumber);
  }
  return result;
}
