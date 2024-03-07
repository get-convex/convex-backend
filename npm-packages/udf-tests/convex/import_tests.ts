import { assert, use } from "chai";
import chaiAsPromised from "chai-as-promised";
import { action, query } from "./_generated/server";

import * as helpersStatic from "./helpers";

export const dynamicImport = action(async () => {
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
});

export const dynamicImportNonexistent = action(async () => {
  use(chaiAsPromised);

  const path = "./nonexistentPath";
  // Note this assertion means that `import("nonexistent")` does not throw an
  // error -- it returns a rejected promise.
  await assert.isRejected(
    import(path),
    Error,
    "Couldn't find JavaScript module 'nonexistentPath'",
  );
});

export const dynamicImportQuery = query(async () => {
  const helpers = await import("./helpers");
  return helpers.fibonacci(6);
});

export const dynamicImportLoadFailure = action(async () => {
  use(chaiAsPromised);
  await assert.isRejected(
    import("./load_failure"),
    Error,
    "Couldn't find JavaScript module 'nonexistentPath'",
  );
});
