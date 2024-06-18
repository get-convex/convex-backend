import { GenericId } from "./index.js";
import { GenericValidator } from "./validator.js";
import { JSONValue, convexToJson } from "./value.js";

type TableNameFromType<T> =
  T extends GenericId<infer TableName> ? TableName : string;

export class VId<Type, IsOptional extends OptionalProperty = "required"> {
  readonly tableName: TableNameFromType<Type>;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "id" as const;
  readonly isConvexValidator: true;
  constructor({
    isOptional,
    tableName,
  }: {
    isOptional: IsOptional;
    tableName: TableNameFromType<Type>;
  }) {
    this.isOptional = isOptional;
    this.tableName = tableName;
    this.isConvexValidator = true;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: "id", tableName: this.tableName };
  }
  /** @internal */
  optional() {
    return new VId<Type, "optional">({
      isOptional: "optional",
      tableName: this.tableName,
    });
  }
}

export class VFloat64<
  Type = number,
  IsOptional extends OptionalProperty = "required",
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "float64" as const;
  readonly isConvexValidator: true;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
    this.isConvexValidator = true;
  }
  /** @internal */
  get json(): ValidatorJSON {
    // Server expects the old name `number`.
    return { type: "number" };
  }
  /** @internal */
  optional() {
    return new VFloat64({ isOptional: "optional" });
  }
}

export class VInt64<
  Type = bigint,
  IsOptional extends OptionalProperty = "required",
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "int64" as const;
  readonly isConvexValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    // Server expects the old name `bigint`.
    return { type: "bigint" };
  }
  /** @internal */
  optional() {
    return new VInt64({ isOptional: "optional" });
  }
}

export class VBoolean<
  Type = boolean,
  IsOptional extends OptionalProperty = "required",
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "boolean" as const;
  isConvexValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new VBoolean({ isOptional: "optional" });
  }
}

export class VBytes<
  Type = ArrayBuffer,
  IsOptional extends OptionalProperty = "required",
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "bytes" as const;
  readonly isConvexValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new VBytes({ isOptional: "optional" });
  }
}

export class VString<
  Type = string,
  IsOptional extends OptionalProperty = "required",
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "string" as const;
  readonly isConvexValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new VString({ isOptional: "optional" });
  }
}

export class VNull<
  Type = null,
  IsOptional extends OptionalProperty = "required",
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "null" as const;
  isConvexValidator: true;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
    this.isConvexValidator = true;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new VNull({ isOptional: "optional" });
  }
}

export class VAny<
  Type = any,
  IsOptional extends OptionalProperty = "required",
  FieldPaths extends string = string,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "any" as const;
  readonly isConvexValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
    };
  }
  /** @internal */
  optional() {
    return new VAny({ isOptional: "optional" });
  }
}

/**
 * Validator for an object that produces indexed types.
 *
 * If the value validator is not optional, it produces a `Record` type, which is an alias
 * for `{[key: K]: V}`.
 *
 * If the value validator is optional, it produces a mapped object type,
 * with optional keys: `{[key in K]?: V}`.
 *
 * This is used within the validator builder, {@link v}.
 */
export class VObject<
  Type,
  Fields extends Record<string, GenericValidator>,
  IsOptional extends OptionalProperty = "required",
  FieldPaths extends string = {
    [Property in keyof Fields]:
      | JoinFieldPaths<Property & string, Fields[Property]["fieldPaths"]>
      | Property;
  }[keyof Fields] &
    string,
> {
  fields: Fields;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "object" as const;
  readonly isConvexValidator = true as const;
  constructor({
    isOptional,
    fields,
  }: {
    isOptional: IsOptional;
    fields: Fields;
  }) {
    this.isOptional = isOptional;
    this.fields = fields;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
      value: globalThis.Object.fromEntries(
        globalThis.Object.entries(this.fields).map(([k, v]) => [
          k,
          {
            fieldType: v.json,
            optional:
              typeof v.isOptional === "boolean"
                ? (this.isOptional as unknown as boolean)
                : v.isOptional === "optional",
          },
        ]),
      ),
    };
  }
  /** @internal */
  optional() {
    return new VObject<Type, Fields, "optional", FieldPaths>({
      isOptional: "optional",
      fields: this.fields,
    });
  }
}

export class VLiteral<Type, IsOptional extends OptionalProperty = "required"> {
  readonly value: Type;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "literal" as const;
  readonly isConvexValidator = true as const;
  constructor({ isOptional, value }: { isOptional: IsOptional; value: Type }) {
    this.isOptional = isOptional;
    this.value = value;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
      value: convexToJson(this.value as string | boolean | number | bigint),
    };
  }
  /** @internal */
  optional() {
    return new VLiteral<Type, "optional">({
      isOptional: "optional",
      value: this.value,
    });
  }
}

export class VArray<
  Type,
  Element extends Validator<any, "required", any>,
  IsOptional extends OptionalProperty = "required",
> {
  element: Element;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "array" as const;
  readonly isConvexValidator = true as const;
  constructor({
    isOptional,
    element,
  }: {
    isOptional: IsOptional;
    element: Element;
  }) {
    this.isOptional = isOptional;
    this.element = element;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
      value: this.element.json,
    };
  }
  /** @internal */
  optional() {
    return new VArray<Type, Element, "optional">({
      isOptional: "optional",
      element: this.element,
    });
  }
}

export class VRecord<
  Type,
  Key extends Validator<string, "required", any>,
  Value extends Validator<any, "required", any>,
  IsOptional extends OptionalProperty = "required",
  FieldPaths extends string = string,
> {
  key: Key;
  value: Value;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "record" as const;
  readonly isConvexValidator = true as const;
  constructor({
    isOptional,
    key,
    value,
  }: {
    isOptional: IsOptional;
    key: Key;
    value: Value;
  }) {
    this.isOptional = isOptional;
    this.key = key;
    this.value = value;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
      keys: this.key.json,
      values: {
        fieldType: this.value.json,
        optional: false,
      },
    };
  }
  /** @internal */
  optional() {
    return new VRecord<Type, Key, Value, "optional">({
      isOptional: "optional",
      key: this.key,
      value: this.value,
    });
  }
}

export class VUnion<
  Type,
  T extends Validator<any, "required", any>[],
  IsOptional extends OptionalProperty = "required",
  FieldPaths extends string = T[number]["fieldPaths"],
> {
  readonly members: T;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "union" as const;
  readonly isConvexValidator = true as const;
  constructor({ isOptional, members }: { isOptional: IsOptional; members: T }) {
    this.isOptional = isOptional;
    this.members = members;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
      value: this.members.map((v) => v.json),
    };
  }
  /** @internal */
  optional() {
    return new VUnion<Type, T, "optional">({
      isOptional: "optional",
      members: this.members,
    });
  }
}

// prettier-ignore
export type VOptional<T extends Validator<any, OptionalProperty, any>> =
  T extends VId<infer Type, OptionalProperty> ? VId<Type, "optional">
  : T extends VString<infer Type, OptionalProperty>
    ? VString<Type, "optional">
  : T extends VFloat64<infer Type, OptionalProperty>
    ? VFloat64<Type, "optional">
  : T extends VInt64<infer Type, OptionalProperty>
    ? VInt64<Type, "optional">
  : T extends VBoolean<infer Type, OptionalProperty>
    ? VBoolean<Type, "optional">
  : T extends VNull<infer Type, OptionalProperty>
    ? VNull<Type, "optional">
  : T extends VAny<infer Type, OptionalProperty>
    ? VAny<Type, "optional">
  : T extends VLiteral<infer Type, OptionalProperty>
    ? VLiteral<Type, "optional">
  : T extends VBytes<infer Type, OptionalProperty>
    ? VBytes<Type, "optional">
  : T extends VObject< infer Type, infer Fields, OptionalProperty, infer FieldPaths>
    ? VObject<Type, Fields, "optional", FieldPaths>
  : T extends VArray<infer Type, infer Element, OptionalProperty>
    ? VArray<Type, Element, "optional">
  : T extends VRecord< infer Type, infer Key, infer Value, OptionalProperty, infer FieldPaths>
    ? VRecord<Type, Key, Value, "optional", FieldPaths>
  : T extends VUnion<infer Type, infer Members, OptionalProperty, infer FieldPaths>
    ? VUnion<Type, Members, "optional", FieldPaths>
  : never

/**
 * Type representing whether a property in an object is optional or required.
 *
 * @public
 */
export type OptionalProperty = "optional" | "required";

/**
 * A validator for a Convex value.
 *
 * This should be constructed using the validator builder, {@link v}.
 *
 * A validator encapsulates:
 * - The TypeScript type of this value.
 * - Whether this field should be optional if it's included in an object.
 * - The TypeScript type for the set of index field paths that can be used to
 * build indexes on this value.
 * - A JSON representation of the validator.
 *
 * Specific types of validators contain additional information: for example
 * an `ArrayValidator` contains an `element` property with the validator
 * used to validate each element of the list. Use the shared 'kind' property
 * to identity the type of validator.
 *
 * More validators can be added in future releases so an exhaustive
 * switch statement on validator `kind` should be expected to break
 * in future releases of Convex.
 *
 * @public
 */
// TODO: Using string for the first IdValidator type param fixed something... right?
// How could this matter? Try reverting all this one and all the others to confirm.
export type Validator<
  Type,
  IsOptional extends OptionalProperty = "required",
  FieldPaths extends string = never,
> =
  | VId<Type, IsOptional>
  | VString<Type, IsOptional>
  | VFloat64<Type, IsOptional>
  | VInt64<Type, IsOptional>
  | VBoolean<Type, IsOptional>
  | VNull<Type, IsOptional>
  | VAny<Type, IsOptional>
  | VLiteral<Type, IsOptional>
  | VBytes<Type, IsOptional>
  | VObject<
      Type,
      Record<string, Validator<any, OptionalProperty, any>>,
      IsOptional,
      FieldPaths
    >
  | VArray<Type, Validator<any, "required", any>, IsOptional>
  | VRecord<
      Type,
      Validator<string, "required", any>,
      Validator<any, "required", any>,
      IsOptional,
      FieldPaths
    >
  | VUnion<Type, Validator<any, "required", any>[], IsOptional, FieldPaths>;

/**
 * Join together two index field paths.
 *
 * This is used within the validator builder, {@link v}.
 * @public
 */
export type JoinFieldPaths<
  Start extends string,
  End extends string,
> = `${Start}.${End}`;

export type ObjectFieldType = { fieldType: ValidatorJSON; optional: boolean };

export type ValidatorJSON =
  | { type: "null" }
  | { type: "number" }
  | { type: "bigint" }
  | { type: "boolean" }
  | { type: "string" }
  | { type: "bytes" }
  | { type: "any" }
  | { type: "literal"; value: JSONValue }
  | { type: "id"; tableName: string }
  | { type: "array"; value: ValidatorJSON }
  | { type: "record"; keys: ValidatorJSON; values: ObjectFieldType }
  | { type: "object"; value: Record<string, ObjectFieldType> }
  | { type: "union"; value: ValidatorJSON[] };
