import { apiCodegen as esmApiCodegen } from "./api.js";
import { GeneratedJsWithTypes, header } from "./common.js";

export function apiCjsCodegen(modulePaths: string[]): GeneratedJsWithTypes {
  const { DTS } = esmApiCodegen(modulePaths);
  const apiJS = `${header("Generated `api` utility.")}
  const { anyApi } = require("convex/server");
  module.exports = {
    api: anyApi,
    internal: anyApi,
  };
  `;
  return {
    DTS,
    JS: apiJS,
  };
}
