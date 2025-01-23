"use node";
import { action } from "../_generated/server";
// Install these on the server via "externalPackages"
const fetch = require("node-fetch");
import * as react from "react";
import * as jest from "jest";

export const getReactVersion = action(async () => {
  // These lines ensure the externalPackages ESBuild plugin marks these dependencies as used
  // so they can be registered as external.
  fetch;
  react;
  jest;
  return react.version;
});
