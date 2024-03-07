import url from "url";
import path from "path";
import fs from "fs";

const __dirname = url.fileURLToPath(new URL(".", import.meta.url));
const convexDir = path.join(__dirname, "..");

assertNoTarballs(convexDir);

// Remove tarballs so there's no confusion about which one was just created.
// The postpack script has to guess, so let's make it explicit.
function assertNoTarballs(dirname) {
  const files = fs.readdirSync(dirname);
  const tarballs = files.filter((f) => f.endsWith(".tgz"));
  for (const tarball of tarballs) {
    fs.rmSync(tarball);
    console.log(`tarball ${tarball} was already present, deleted.`);
  }
}
