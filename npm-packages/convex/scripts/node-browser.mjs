/**
 * Create two new entry points for convex/browser, one just for Node.js.
 *
 * The Node.js build includes in a WebSocket implementation and
 * attempts to dynamically import node-fetch in Node.js versions
 * that don't have native fetch support (<18) so that node-fetch
 * doesn't need to be bundled.
 */
import url from "url";
import path from "path";
import fs from "fs";

const [tempDir] = process.argv
  .filter((arg) => arg.startsWith("tempDir="))
  .map((arg) => arg.slice(8));

const __dirname = url.fileURLToPath(new URL(".", import.meta.url));
const convexDir = path.join(__dirname, "..");
const distDir = path.join(convexDir, tempDir);
const cjsBrowserIndex = path.join(distDir, "cjs", "browser", "index.js");
const esmBrowserIndex = path.join(distDir, "esm", "browser", "index.js");
const cjsBrowserIndexNode = path.join(
  distDir,
  "cjs",
  "browser",
  "index-node.js",
);
const esmBrowserIndexNode = path.join(
  distDir,
  "esm",
  "browser",
  "index-node.js",
);

let output = fs.readFileSync(cjsBrowserIndex, { encoding: "utf-8" });
output = output.replace('"./http_client.js"', '"./http_client-node.js"');
output = output.replace('"./simple_client.js"', '"./simple_client-node.js"');
fs.writeFileSync(cjsBrowserIndexNode, output, { encoding: "utf-8" });

output = fs.readFileSync(esmBrowserIndex, { encoding: "utf-8" });
output = output.replace('"./http_client.js"', '"./http_client-node.js"');
output = output.replace('"./simple_client.js"', '"./simple_client-node.js"');
fs.writeFileSync(esmBrowserIndexNode, output, { encoding: "utf-8" });
