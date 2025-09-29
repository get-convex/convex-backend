#!/usr/bin/env node
// Until https://github.com/colinhacks/zshy/issues/34 is resolved show a nicer error for the wrong Node.js version

const v = process.versions.node;
const [maj, min, pat] = v.split(".").map(Number);

if (
  maj < 20 ||
  (maj === 20 && min < 19) ||
  (maj === 22 && (min < 12 || (min === 12 && pat < 0)))
) {
  console.error(
    `Node.js ${v} does not support synchronous require() of ESM. Need 20.19.0+, 22.12.0+, or 23+`,
  );
  process.exit(1);
}
