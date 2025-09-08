import { createRequire } from "module";

const require2 = createRequire(import.meta.url);
const packageJson = require2("../package.json");

export const version = packageJson.version;
