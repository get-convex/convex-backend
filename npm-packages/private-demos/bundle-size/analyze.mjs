#!/usr/bin/env node
import fs from "fs";

// prettier-ignore
const REACT_EST = (0
  + 6000  // react (split the difference between v17 and v18)
  + 125000 // react-dom
);

function format(number) {
  if (number < 1000) return `${number} bytes`;
  if (number < 10000) return `${Math.ceil(number / 10) / 100} kilobytes`;
  if (number < 100000) return `${Math.ceil(number / 100) / 10} kilobytes`;
  if (number < 1000000) return `${Math.ceil(number / 1000)} kilobytes`;
  return number + " bytes";
}

const esbuild = JSON.parse(fs.readFileSync("dist/esbuild.json"));
const bundle = esbuild.outputs["dist/esbuild-output"];
console.log(
  `esbuild: ${bundle.bytes} bytes       `.slice(0, 23),
  "without react est.",
  format(bundle.bytes - REACT_EST),
);

const parcelName = fs
  .readdirSync("dist/assets/")
  .filter((fn) => fn.endsWith("js"));
const parcelOutputBytes = fs.statSync(`dist/assets/${parcelName}`).size;
console.log(
  `parcel:  ${parcelOutputBytes} bytes      `.slice(0, 23),
  "without react est.",
  format(parcelOutputBytes - REACT_EST),
);

const webpackOutputBytes = fs.statSync(`dist/webpack.bundle.js`).size;
console.log(
  `webpack: ${webpackOutputBytes} bytes      `.slice(0, 23),
  "without react est.",
  format(webpackOutputBytes - REACT_EST),
);

console.log(
  format((bundle.bytes + parcelOutputBytes + webpackOutputBytes) / 3),
);
