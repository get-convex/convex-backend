"use node";

import {
  assert,
  // Renaming the import to avoid react-hooks/rules-of-hooks
  use as chaiUse,
} from "chai";
import chaiAsPromised from "chai-as-promised";
import { action } from "./_generated/server";

import * as helpersStatic from "./helpers";

export const dynamicImport = action({
  args: {},
  handler: async () => {
    const helpers = await import("./helpers");
    assert.strictEqual(helpers.fibonacci(6), 8.0);
    // The `import * as helpersStatic` repackages the namespace object, so we
    // can't assert helpersStatic === helpers, but we can check that the module
    // was not re-evaluated by checking equality of a field.
    assert.strictEqual(helpersStatic.fibonacci, helpers.fibonacci);
    const helpersAgain = await import("./helpers");
    assert.strictEqual(helpers, helpersAgain);
    const helpersDifferentPath = await import("./helpers");
    assert.strictEqual(helpers, helpersDifferentPath);
  },
});

export const dynamicImportNonexistent = action({
  args: {},
  handler: async () => {
    chaiUse(chaiAsPromised);

    const path = "./nonexistentPath";
    // Note this assertion means that `import("nonexistent")` does not throw an
    // error -- it returns a rejected promise.
    await assert.isRejected(
      import(path),
      Error,
      "Couldn't find JavaScript module 'nonexistentPath'",
    );
  },
});

// NOTE: dynamicImportQuery was removed because dynamic imports in V8 isolate
// files (queries/mutations without "use node") are now blocked at build time.
// The test that verified runtime failure is no longer possible to build.
// See test_query_dynamic_import in crates/isolate/src/tests/import.rs for the
// corresponding Rust test that also needs to be removed.

export const dynamicImportLoadFailure = action({
  args: {},
  handler: async () => {
    chaiUse(chaiAsPromised);
    await assert.isRejected(
      import("./load_failure"),
      Error,
      "Couldn't find JavaScript module 'nonexistentPath'",
    );
  },
});
