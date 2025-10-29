"use node";
import { action } from "../_generated/server";
// Install these on the server via "externalPackages"
// eslint-disable-next-line @typescript-eslint/no-require-imports
const fetch = require("node-fetch");
import * as react from "react";
import * as jest from "jest";

export const getReactVersion = action(async () => {
  // These lines ensure the externalPackages ESBuild plugin marks these dependencies as used
  // so they can be registered as external.
  // eslint-disable-next-line @typescript-eslint/no-unused-expressions
  fetch;
  // eslint-disable-next-line @typescript-eslint/no-unused-expressions
  react;
  // eslint-disable-next-line @typescript-eslint/no-unused-expressions
  jest;
  return react.version;
});
