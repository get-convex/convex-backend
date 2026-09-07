import { convexToJson, JSONValue, Value } from "../../values/value.js";

export function validatorSpecToJson(validator: Value): JSONValue {
  if (typeof validator === "string") {
    return JSON.parse(validator) as JSONValue;
  }
  return convexToJson(validator);
}

export function apiSpecFunctionsToJson(functions: Value): JSONValue {
  if (!Array.isArray(functions)) {
    return convexToJson(functions);
  }
  return functions.map((fn) => {
    if (typeof fn !== "object" || fn === null || Array.isArray(fn)) {
      return convexToJson(fn);
    }
    if (fn.functionType === "HttpAction") {
      return convexToJson(fn);
    }
    const out: { [key: string]: JSONValue } = {};
    if (fn.identifier !== undefined) {
      out.identifier = convexToJson(fn.identifier);
    }
    if (fn.functionType !== undefined) {
      out.functionType = convexToJson(fn.functionType);
    }
    if (fn.visibility !== undefined) {
      out.visibility = convexToJson(fn.visibility);
    }
    if (fn.args !== undefined) {
      out.args = validatorSpecToJson(fn.args);
    }
    if (fn.returns !== undefined) {
      out.returns = validatorSpecToJson(fn.returns);
    }
    return out;
  });
}
