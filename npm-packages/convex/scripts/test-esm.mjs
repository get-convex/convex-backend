// Import everything as ESM to make sure our imports are valid ESM
// (. -> ./index.js, ./foo -> ./foo.js, ./foo.ts -> ./foo.js

import fs from "fs";
import path, { dirname } from "path";
import { fileURLToPath } from "url";
const __dirname = dirname(fileURLToPath(import.meta.url));

await import("../dist/esm/index.js");

for (const dir of fs.readdirSync(path.join(__dirname, "../dist/esm"))) {
  if (dir.endsWith("cli")) {
    // CLI is tested elsewhere, importing it here exits the process
    continue;
  }

  const index = path.join("../dist/esm", dir, "index.js");
  const indexAbsolute = path.join(__dirname, index);
  if (fs.existsSync(indexAbsolute)) {
    await import(index);
  }
}
