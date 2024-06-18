import { Expand } from "../type_utils.js";
import { GenericId } from "./index.js";
import {
  OptionalProperty,
  VAny,
  VArray,
  VBoolean,
  VBytes,
  VFloat64,
  VId,
  VInt64,
  VLiteral,
  VNull,
  VObject,
  VOptional,
  VRecord,
  VString,
  VUnion,
  Validator,
} from "./validators.js";

/**
 * The type that all validators must extend.
 *
 * @public
 */
export type GenericValidator = Validator<any, any, any>;

export function isValidator(v: any): v is GenericValidator {
  return !!v.isValidator;
}

/**
 * The validator builder.
 *
 * This builder allows you to build validators for Convex values.
 *
 * Validators can be used in [schema definitions](https://docs.convex.dev/database/schemas)
 * and as input validators for Convex functions.
 *
 * @public
 */
export const v = {
  id<TableName extends string>(tableName: TableName) {
    return new VId<GenericId<TableName>>({
      isOptional: "required",
      tableName,
    });
  },
  null() {
    return new VNull({ isOptional: "required" });
  },
  /**
   * Alias for `v.float64()`
   */
  number() {
    return new VFloat64({ isOptional: "required" });
  },
  float64() {
    return new VFloat64({ isOptional: "required" });
  },
  /**
   * @deprecated Use `v.int64()` instead
   */
  bigint() {
    return new VInt64({ isOptional: "required" });
  },
  int64() {
    return new VInt64({ isOptional: "required" });
  },
  boolean() {
    return new VBoolean({ isOptional: "required" });
  },
  string() {
    return new VString({ isOptional: "required" });
  },
  bytes() {
    return new VBytes({ isOptional: "required" });
  },
  literal<T extends string | number | bigint | boolean>(literal: T) {
    return new VLiteral<T>({ isOptional: "required", value: literal });
  },
  array<T extends Validator<any, "required", any>>(element: T) {
    return new VArray<T["type"][], T>({ isOptional: "required", element });
  },
  object<T extends PropertyValidators>(fields: T) {
    return new VObject<ObjectType<T>, T>({ isOptional: "required", fields });
  },

  /** @internal */
  record<
    Key extends Validator<any, "required", any>,
    Value extends Validator<any, "required", any>,
  >(keys: Key, values: Value) {
    // TODO enforce that Infer<key> extends string
    return new VRecord<
      Value["isOptional"] extends true
        ? { [key in Infer<Key>]?: Value["type"] }
        : Record<Infer<Key>, Value["type"]>,
      Key,
      Value
    >({
      isOptional: "required",
      key: keys,
      value: values,
    });
  },

  union<T extends Validator<any, "required", any>[]>(...members: T) {
    return new VUnion<T[number]["type"], T>({
      isOptional: "required",
      members,
    });
  },
  any() {
    return new VAny({ isOptional: "required" });
  },
  optional<T extends GenericValidator>(value: T) {
    return value.optional() as VOptional<T>;
  },
};

/**
 * Validators for each property of an object.
 *
 * This is represented as an object mapping the property name to its
 * {@link Validator}.
 *
 * @public
 */
export type PropertyValidators = Record<
  string,
  Validator<any, OptionalProperty, any>
>;

/**
 * Compute the type of an object from {@link PropertyValidators}.
 *
 * @public
 */
export type ObjectType<Fields extends PropertyValidators> = Expand<
  // Map each key to the corresponding property validator's type making
  // the optional ones optional.
  {
    [Property in OptionalKeys<Fields>]?: Infer<Fields[Property]>;
  } & {
    [Property in RequiredKeys<Fields>]: Infer<Fields[Property]>;
  }
>;

type OptionalKeys<PropertyValidators extends Record<string, GenericValidator>> =
  {
    [Property in keyof PropertyValidators]: PropertyValidators[Property]["isOptional"] extends "optional"
      ? Property
      : never;
  }[keyof PropertyValidators];

type RequiredKeys<PropertyValidators extends Record<string, GenericValidator>> =
  Exclude<keyof PropertyValidators, OptionalKeys<PropertyValidators>>;

/**
 * Extract a TypeScript type from a validator.
 *
 * Example usage:
 * ```ts
 * const objectSchema = v.object({
 *   property: v.string(),
 * });
 * type MyObject = Infer<typeof objectSchema>; // { property: string }
 * ```
 * @typeParam V - The type of a {@link Validator} constructed with {@link v}.
 *
 * @public
 */
export type Infer<T extends Validator<any, OptionalProperty, any>> = T["type"];
