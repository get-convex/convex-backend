import { test, expect } from "@jest/globals";
import { oneoffContext } from "./context.js";

// Although these tests are run as ESM by ts-lint, this file is built as both
// CJS and ESM by TypeScript so normal recipes like `__dirname` for getting the
// script directory don't work.
const dirname = "src/bundler";

import {
  bundle,
  entryPointsByEnvironment,
  useNodeDirectiveRegex,
  mustBeIsolate,
} from "./index.js";

const sorted = <T>(arr: T[], key: (el: T) => any): T[] => {
  const newArr = [...arr];
  const cmp = (a: T, b: T) => {
    if (key(a) < key(b)) return -1;
    if (key(a) > key(b)) return 1;
    return 0;
  };
  return newArr.sort(cmp);
};

test("bundle function is present", () => {
  expect(typeof bundle).toEqual("function");
});

test("bundle finds JavaScript functions", async () => {
  const entryPoints = await entryPointsByEnvironment(
    oneoffContext,
    dirname + "/test_fixtures/js",
    false,
  );
  const bundles = sorted(
    (
      await bundle(
        oneoffContext,
        dirname + "/test_fixtures/js",
        entryPoints.isolate,
        false,
        "browser",
      )
    ).modules,
    (b) => b.path,
  ).filter((bundle) => !bundle.path.includes("_deps"));
  expect(bundles).toHaveLength(2);
  expect(bundles[0].path).toEqual("bar.js");
  expect(bundles[1].path).toEqual("foo.js");
});

test("use node regex", () => {
  // Double quotes
  expect('"use node";').toMatch(useNodeDirectiveRegex);
  // Single quotes
  expect("'use node';").toMatch(useNodeDirectiveRegex);
  // No semi column
  expect('"use node"').toMatch(useNodeDirectiveRegex);
  expect("'use node'").toMatch(useNodeDirectiveRegex);
  // Extra spaces
  expect('   "use node"   ').toMatch(useNodeDirectiveRegex);
  expect("   'use node'   ").toMatch(useNodeDirectiveRegex);

  // Nothing
  expect("").not.toMatch(useNodeDirectiveRegex);
  // No quotes
  expect("use node").not.toMatch(useNodeDirectiveRegex);
  // In a comment
  expect('// "use node";').not.toMatch(useNodeDirectiveRegex);
  // Typo
  expect('"use nod";').not.toMatch(useNodeDirectiveRegex);
  // Extra quotes
  expect('""use node"";').not.toMatch(useNodeDirectiveRegex);
  expect("''use node'';").not.toMatch(useNodeDirectiveRegex);
  // Extra semi colons
  expect('"use node";;;').not.toMatch(useNodeDirectiveRegex);
  // Twice
  expect('"use node";"use node";').not.toMatch(useNodeDirectiveRegex);
});

test("must use isolate", () => {
  expect(mustBeIsolate("http.js")).toBeTruthy();
  expect(mustBeIsolate("http.mjs")).toBeTruthy();
  expect(mustBeIsolate("http.ts")).toBeTruthy();
  expect(mustBeIsolate("crons.js")).toBeTruthy();
  expect(mustBeIsolate("crons.cjs")).toBeTruthy();
  expect(mustBeIsolate("crons.ts")).toBeTruthy();
  expect(mustBeIsolate("schema.js")).toBeTruthy();
  expect(mustBeIsolate("schema.jsx")).toBeTruthy();
  expect(mustBeIsolate("schema.ts")).toBeTruthy();

  expect(mustBeIsolate("http.sample.js")).not.toBeTruthy();
  expect(mustBeIsolate("https.js")).not.toBeTruthy();
  expect(mustBeIsolate("schema2.js")).not.toBeTruthy();
  expect(mustBeIsolate("schema/http.js")).not.toBeTruthy();
});
