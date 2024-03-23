import { JSONValue, convexToJson, Id as GenericId } from "./value.js";
import { Expand } from "../type_utils.js";

/**
 * A validator for a Convex value.
 *
 * This should be constructed using the validator builder, {@link v}.
 *
 * This class encapsulates:
 * - The TypeScript type of this value.
 * - Whether this field should be optional if it's included in an object.
 * - The TypeScript type for the set of index field paths that can be used to
 * build indexes on this value.
 * - A JSON representation of the validator.
 * @public
 */
export class Validator<
  TypeScriptType,
  IsOptional extends boolean = false,
  FieldPaths extends string = never,
> {
  readonly type!: TypeScriptType;
  readonly isOptional!: IsOptional;
  readonly fieldPaths!: FieldPaths;

  // Property for a bit of nominal type safety.
  readonly _isValidator: undefined;

  readonly optional: boolean;
  /**
   * @internal
   */
  readonly json: ValidatorJSON;
  /**
   * @internal
   */
  constructor(json: ValidatorJSON, optional: boolean) {
    this.json = json;
    this.optional = optional;
  }
}

/**
 * @internal
 */
export type ObjectFieldType = { fieldType: ValidatorJSON; optional: boolean };

/**
 * @internal
 */
export type ValidatorJSON =
  | {
      type: "null";
    }
  | { type: "number" }
  | { type: "bigint" }
  | { type: "boolean" }
  | { type: "string" }
  | { type: "bytes" }
  | { type: "any" }
  | {
      type: "literal";
      value: JSONValue;
    }
  | { type: "id"; tableName: string }
  | { type: "array"; value: ValidatorJSON }
  | { type: "record"; keys: ValidatorJSON; values: ObjectFieldType }
  | { type: "object"; value: Record<string, ObjectFieldType> }
  | { type: "union"; value: ValidatorJSON[] };

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
  id<TableName extends string>(
    tableName: TableName,
  ): Validator<GenericId<TableName>> {
    return new Validator({ type: "id", tableName }, false);
  },
  null(): Validator<null> {
    return new Validator({ type: "null" }, false);
  },
  /**
   * Alias for `v.float64()`
   */
  number(): Validator<number> {
    return new Validator({ type: "number" }, false);
  },
  float64(): Validator<number> {
    return new Validator({ type: "number" }, false);
  },
  /**
   * @deprecated Use `v.int64()` instead
   */
  bigint(): Validator<bigint> {
    return new Validator({ type: "bigint" }, false);
  },
  int64(): Validator<bigint> {
    return new Validator({ type: "bigint" }, false);
  },
  boolean(): Validator<boolean> {
    return new Validator({ type: "boolean" }, false);
  },
  string(): Validator<string> {
    return new Validator({ type: "string" }, false);
  },
  bytes(): Validator<ArrayBuffer> {
    return new Validator({ type: "bytes" }, false);
  },
  literal<T extends string | number | bigint | boolean>(
    literal: T,
  ): Validator<T> {
    const value = convexToJson(literal);
    return new Validator({ type: "literal", value }, false);
  },
  array<T>(values: Validator<T, false, any>): Validator<T[]> {
    return new Validator({ type: "array", value: values.json }, false);
  },
  object<T extends PropertyValidators>(schema: T): ObjectValidator<T> {
    return new Validator(
      {
        type: "object",
        value: Object.fromEntries(
          Object.entries(schema).map(([k, v]) => [
            k,
            { fieldType: v.json, optional: v.optional },
          ]),
        ),
      },
      false,
    );
  },

  /** @internal */
  record<K extends string, ValueValidator extends Validator<any, any, any>>(
    keys: Validator<K, false, any>,
    values: ValueValidator,
  ): RecordValidator<K, ValueValidator> {
    return new Validator(
      {
        type: "record",
        keys: keys.json,
        values: { fieldType: values.json, optional: values.optional },
      },
      false,
    );
  },

  union<
    T extends [
      Validator<any, false, any>,
      Validator<any, false, any>,
      ...Validator<any, false, any>[],
    ],
  >(
    ...schemaTypes: T
  ): Validator<T[number]["type"], false, T[number]["fieldPaths"]> {
    return new Validator(
      {
        type: "union",
        value: schemaTypes.map((t) => t.json),
      },
      false,
    );
  },
  any(): Validator<any, false, string> {
    return new Validator({ type: "any" }, false);
  },
  optional<T extends Validator<any, false, any>>(
    inner: T,
  ): Validator<T["type"] | undefined, true, T["fieldPaths"]> {
    return new Validator(inner.json, true) as Validator<
      T["type"],
      true,
      T["fieldPaths"]
    >;
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
export type PropertyValidators = Record<string, Validator<any, any, any>>;

/**
 * Compute the type of an object from {@link PropertyValidators}.
 *
 * @public
 */
export type ObjectType<Validators extends PropertyValidators> = Expand<
  // Map each key to the corresponding property validator's type making
  // the optional ones optional.
  {
    [Property in OptionalKeys<Validators>]?: Validators[Property]["type"];
  } & {
    [Property in RequiredKeys<Validators>]: Validators[Property]["type"];
  }
>;

/**
 * Calculate the type of a {@link Validator} for an object.
 *
 * This is used within the validator builder, {@link v}.
 *
 * @public
 */
export type ObjectValidator<Validators extends PropertyValidators> = Validator<
  // Compute the TypeScript type this validator refers to.
  ObjectType<Validators>,
  false,
  // Compute the field paths for this validator. For every property in the object,
  // add on a field path for that property and extend all the field paths in the
  // validator.
  {
    [Property in keyof Validators]:
      | JoinFieldPaths<Property & string, Validators[Property]["fieldPaths"]>
      | Property;
  }[keyof Validators] &
    string
>;

type OptionalKeys<
  PropertyValidators extends Record<string, Validator<any, any, any>>,
> = {
  [Property in keyof PropertyValidators]: PropertyValidators[Property]["isOptional"] extends true
    ? Property
    : never;
}[keyof PropertyValidators];

type RequiredKeys<
  PropertyValidators extends Record<string, Validator<any, any, any>>,
> = Exclude<keyof PropertyValidators, OptionalKeys<PropertyValidators>>;

/**
 * Calculate the type of a {@link Validator} for an object that produces indexed types.
 *
 * If the value validator is not optional, it produces a `Record` type, which is an alias
 * for `{[key: K]: V}`.
 *
 * If the value validator is optional, it produces a mapped object type,
 * with optional keys: `{[key in K]?: V}`.
 *
 * This is used within the validator builder, {@link v}.
 */
export type RecordValidator<
  K extends string,
  ValueValidator extends Validator<any, any, any>,
> = Validator<
  ValueValidator["isOptional"] extends true
    ? { [key in K]?: ValueValidator["type"] }
    : Record<K, ValueValidator["type"]>
>;

/**
 * Join together two index field paths.
 *
 * This is used within the validator builder, {@link v}.
 * @public
 */
type JoinFieldPaths<
  Start extends string,
  End extends string,
> = `${Start}.${End}`;

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
export type Infer<V extends Validator<any, any, any>> = V["type"];
