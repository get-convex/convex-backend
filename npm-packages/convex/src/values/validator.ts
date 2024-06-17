import { Expand } from "../type_utils.js";
import { GenericId } from "./index.js";

// Binding this module to `V` makes autocompletion pretty crummy in this file.
import {
  AnyValidator,
  ArrayValidator,
  BooleanValidator,
  BytesValidator,
  Float64Validator,
  IdValidator,
  Int64Validator,
  LiteralValidator,
  NullValidator,
  ObjectValidator,
  OptionalValidator,
  RecordValidator,
  StringValidator,
  UnionValidator,
  Validator,
} from "./validators.js";

export function isValidator(v: any): v is Validator<any, boolean, any> {
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
    return new IdValidator<GenericId<TableName>, TableName>({
      isOptional: false,
      tableName,
    });
  },
  null() {
    return new NullValidator({ isOptional: false });
  },
  /**
   * Alias for `v.float64()`
   */
  number() {
    return new Float64Validator({ isOptional: false });
  },
  float64() {
    return new Float64Validator({ isOptional: false });
  },
  /**
   * @deprecated Use `v.int64()` instead
   */
  bigint() {
    return new Int64Validator({ isOptional: false });
  },
  int64() {
    return new Int64Validator({ isOptional: false });
  },
  boolean() {
    return new BooleanValidator({ isOptional: false });
  },
  string() {
    return new StringValidator({ isOptional: false });
  },
  bytes() {
    return new BytesValidator({ isOptional: false });
  },
  // this could be expanded for more kinds of literals
  literal<T extends string | number | bigint | boolean>(literal: T) {
    return new LiteralValidator<T, T>({ isOptional: false, value: literal });
  },
  array<T extends Validator<any, false, any>>(element: T) {
    return new ArrayValidator<T["type"][], T>({ isOptional: false, element });
  },
  object<T extends PropertyValidators>(fields: T) {
    return new ObjectValidator<ObjectType<T>, T>({ isOptional: false, fields });
  },

  /** @internal */
  record<
    Key extends Validator<any, boolean, any>,
    Value extends Validator<any, boolean, any>,
  >(keys: Key, values: Value) {
    // TODO enforce that Infer<key> extends string
    return new RecordValidator<
      Value["isOptional"] extends true
        ? { [key in Infer<Key>]?: Value["type"] }
        : Record<Infer<Key>, Value["type"]>,
      Key,
      Value
    >({
      isOptional: false,
      key: keys,
      value: values,
    });
  },

  union<T extends Validator<any, false, any>[]>(...members: T) {
    return new UnionValidator<T[number]["type"], T>({
      isOptional: false,
      members,
    });
  },
  any() {
    return new AnyValidator({ isOptional: false });
  },
  optional<T extends Validator<any, boolean, any>>(value: T) {
    return value.optional() as OptionalValidator<T>;
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
export type PropertyValidators = Record<string, Validator<any, boolean, any>>;

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

type OptionalKeys<
  PropertyValidators extends Record<string, Validator<any, boolean, any>>,
> = {
  [Property in keyof PropertyValidators]: PropertyValidators[Property]["isOptional"] extends true
    ? Property
    : never;
}[keyof PropertyValidators];

type RequiredKeys<
  PropertyValidators extends Record<string, Validator<any, boolean, any>>,
> = Exclude<keyof PropertyValidators, OptionalKeys<PropertyValidators>>;

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
export type Infer<T extends Validator<any, boolean, any>> = T["type"];
