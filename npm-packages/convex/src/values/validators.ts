import { JSONValue, convexToJson } from "./value.js";

export class IdValidator<
  Type,
  TableName extends string,
  IsOptional extends boolean = false,
> {
  readonly tableName: TableName;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  // TODO are these fieldpaths necessary?
  readonly fieldPaths!: never;
  readonly kind = "id" as const;
  readonly isValidator: true;
  constructor({
    isOptional,
    tableName,
  }: {
    isOptional: IsOptional;
    tableName: TableName;
  }) {
    this.isOptional = isOptional;
    this.tableName = tableName;
    this.isValidator = true;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: "id", tableName: this.tableName };
  }
  /** @internal */
  optional() {
    return new IdValidator<Type, TableName, true>({
      isOptional: true,
      tableName: this.tableName,
    });
  }
}

export class Float64Validator<
  Type = number,
  IsOptional extends boolean = false,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "float64" as const;
  readonly isValidator: true;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
    this.isValidator = true;
  }
  /** @internal */
  get json(): ValidatorJSON {
    // Server expects the old name `number`.
    return { type: "number" };
  }
  /** @internal */
  optional() {
    return new Float64Validator({ isOptional: true });
  }
}

export class Int64Validator<Type = bigint, IsOptional extends boolean = false> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "int64" as const;
  readonly isValidator = true as const;
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
    return new Int64Validator({ isOptional: true });
  }
}

export class BooleanValidator<
  Type = boolean,
  IsOptional extends boolean = false,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "boolean" as const;
  isValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new BooleanValidator({ isOptional: true });
  }
}

export class BytesValidator<
  Type = ArrayBuffer,
  IsOptional extends boolean = false,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "bytes" as const;
  readonly isValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new BytesValidator({ isOptional: true });
  }
}

export class StringValidator<
  Type = string,
  IsOptional extends boolean = false,
  FieldPaths extends string = never,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "string" as const;
  readonly isValidator = true as const;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new StringValidator({ isOptional: true });
  }
}

export class NullValidator<
  Type = null,
  IsOptional extends boolean = false,
  FieldPaths extends string = never,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "null" as const;
  isValidator: true;
  constructor({ isOptional }: { isOptional: IsOptional }) {
    this.isOptional = isOptional;
    this.isValidator = true;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return { type: this.kind };
  }
  /** @internal */
  optional() {
    return new NullValidator({ isOptional: true });
  }
}

export class AnyValidator<
  Type = any,
  IsOptional extends boolean = false,
  FieldPaths extends string = string,
> {
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "any" as const;
  readonly isValidator = true as const;
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
    return new AnyValidator({ isOptional: true });
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
export class ObjectValidator<
  Type,
  Fields extends Record<string, Validator<any, boolean, any>>,
  IsOptional extends boolean = false,
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
  readonly isValidator = true as const;
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
          { fieldType: v.json, optional: v.isOptional },
        ]),
      ),
    };
  }
  /** @internal */
  optional() {
    return new ObjectValidator<Type, Fields, true, FieldPaths>({
      isOptional: true,
      fields: this.fields,
    });
  }
}

export class LiteralValidator<
  Type,
  Value extends string | number | bigint | boolean,
  IsOptional extends boolean = false,
> {
  readonly value: Value;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: never;
  readonly kind = "literal" as const;
  readonly isValidator = true as const;
  constructor({ isOptional, value }: { isOptional: IsOptional; value: Value }) {
    this.isOptional = isOptional;
    this.value = value;
  }
  /** @internal */
  get json(): ValidatorJSON {
    return {
      type: this.kind,
      value: convexToJson(this.value),
    };
  }
  /** @internal */
  optional() {
    return new LiteralValidator<Type, Value, true>({
      isOptional: true,
      value: this.value,
    });
  }
}

export class ArrayValidator<
  Type,
  Element extends Validator<any, false, any>,
  IsOptional extends boolean = false,
  FieldPaths extends string = never,
> {
  element: Element;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "array" as const;
  readonly isValidator = true as const;
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
    return new ArrayValidator<Type, Element, true>({
      isOptional: true,
      element: this.element,
    });
  }
}

export class RecordValidator<
  Type,
  Key extends Validator<string, boolean, any>,
  Value extends Validator<any, boolean, any>,
  IsOptional extends boolean = false,
  FieldPaths extends string = never,
> {
  key: Key;
  value: Value;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "record" as const;
  readonly isValidator = true as const;
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
        optional: this.value.isOptional,
      },
    };
  }
  /** @internal */
  optional() {
    return new RecordValidator<Type, Key, Value, true>({
      isOptional: true,
      key: this.key,
      value: this.value,
    });
  }
}

export class UnionValidator<
  Type,
  T extends Validator<any, false, any>[],
  IsOptional extends boolean = false,
  FieldPaths extends string = T[number]["fieldPaths"],
> {
  readonly members: T;
  readonly type!: Type;
  readonly isOptional: IsOptional;
  readonly fieldPaths!: FieldPaths;
  readonly kind = "union" as const;
  readonly isValidator = true as const;
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
    return new UnionValidator<Type, T, true>({
      isOptional: true,
      members: this.members,
    });
  }
}

// prettier-ignore
export type OptionalValidator<T extends Validator<any, boolean, any>> =
  T extends IdValidator<infer Type, infer TableName, boolean> ? IdValidator<Type, TableName, true>
  : T extends StringValidator<infer Type, boolean>
    ? StringValidator<Type, true>
  : T extends Float64Validator<infer Type, boolean>
    ? Float64Validator<Type, true>
  : T extends Int64Validator<infer Type, boolean>
    ? Int64Validator<Type, true>
  : T extends BooleanValidator<infer Type, boolean>
    ? BooleanValidator<Type, true>
  : T extends NullValidator<infer Type, boolean>
    ? NullValidator<Type, true>
  : T extends AnyValidator<infer Type, boolean>
    ? AnyValidator<Type, true>
  : T extends LiteralValidator<infer Type, infer Value, boolean>
    ? LiteralValidator<Type, Value, true>
  : T extends BytesValidator<infer Type, boolean>
    ? BytesValidator<Type, true>
  : T extends ObjectValidator< infer Type, infer Fields, boolean, infer FieldPaths>
    ? ObjectValidator<Type, Fields, true, FieldPaths>
  : T extends ArrayValidator<infer Type, infer Element, boolean, infer FieldPaths>
    ? ArrayValidator<Type, Element, true, FieldPaths>
  : T extends RecordValidator< infer Type, infer Key, infer Value, boolean, infer FieldPaths>
    ? RecordValidator<Type, Key, Value, true, FieldPaths>
  : T extends UnionValidator<infer Type, infer Members, boolean, infer FieldPaths>
    ? UnionValidator<Type, Members, true, FieldPaths>
  : never

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
  IsOptional extends boolean = false,
  FieldPaths extends string = never,
> =
  | IdValidator<Type, string, IsOptional>
  | StringValidator<Type, IsOptional>
  | Float64Validator<Type, IsOptional>
  | Int64Validator<Type, IsOptional>
  | BooleanValidator<Type, IsOptional>
  | NullValidator<Type, IsOptional>
  | AnyValidator<Type, IsOptional>
  | LiteralValidator<Type, string | number | bigint | boolean, IsOptional>
  | BytesValidator<Type, IsOptional>
  | ObjectValidator<
      Type,
      Record<string, Validator<any, boolean, any>>,
      IsOptional,
      FieldPaths
    >
  | ArrayValidator<Type, Validator<any, false, any>, IsOptional, FieldPaths>
  | RecordValidator<
      Type,
      Validator<any, boolean, any>,
      Validator<any, boolean, any>,
      IsOptional,
      FieldPaths
    >
  | UnionValidator<Type, Validator<any, false, any>[], IsOptional, FieldPaths>;

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
