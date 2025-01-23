// This file contains tests for specific docs regressions.

const fs = require("fs"); // eslint-disable-line

// Docs contain docstrings, which requires typedoc-plugin-markdown and typedoc to play nice.
// https://linear.app/convex/issue/CX-1825/docstrings-broken-in-docs
const builtDoc = "build/api/modules/react/index.html";
const doc = fs.readFileSync(builtDoc, { encoding: "utf-8" });
const needle = "This module contains:";
if (!doc.includes(needle)) {
  throw new Error(
    `File ${builtDoc} does not contain '${needle}'. Maybe the 'typedoc' peer dependency of typedoc-plugin-markdown is not fulfilled? (or maybe the docs changed)`,
  );
}

// Check that typeParams specifically make it into the docs; this broke in typedoc-plugin-markdown
// https://github.com/tgreyuk/typedoc-plugin-markdown/issues/326
const builtDoc2 = "build/api/classes/server.Expression/index.html";
const doc2 = fs.readFileSync(builtDoc2, { encoding: "utf-8" });
const needle2 = "The type that this expression evaluates to.";
if (!doc2.includes(needle2)) {
  throw new Error(
    `File ${builtDoc2} does not contain '${needle2}'. Maybe this stoped working again (or maybe the docs changed, in which case update this test)`,
  );
}

console.log("docs regression spot-check passed");
