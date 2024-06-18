/**
 * Utilities for working with values stored in Convex.
 *
 * You can see the full set of supported types at
 * [Types](https://docs.convex.dev/using/types).
 * @module
 */

export { convexToJson, jsonToConvex } from "./value.js";
export type {
  Id as GenericId,
  JSONValue,
  Value,
  NumericValue,
} from "./value.js";
export { v } from "./validator.js";
export type {
  PropertyValidators,
  ObjectType,
  GenericValidator,
} from "./validator.js";
export type {
  ValidatorJSON,
  ObjectFieldType,
  Validator,
  OptionalProperty,
} from "./validators.js";
import * as Base64 from "./base64.js";
export { Base64 };
export type { Infer } from "./validator.js";
export * from "./errors.js";
