/* eslint-disable */
/**
 * Generated data model types.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type {
  DocumentByName,
  TableNamesInDataModel,
  SystemTableNames,
  AnyDataModel,
} from "convex/server";
import type { GenericId } from "convex/values";

/**
 * A type describing your Convex data model.
 *
 * This type includes information about what tables you have, the type of
 * documents stored in those tables, and the indexes defined on them.
 *
 * This type is used to parameterize methods like `queryGeneric` and
 * `mutationGeneric` to make them type-safe.
 */

export type DataModel = {
  empty: {
    document: { _id: Id<"empty">; _creationTime: number };
    fieldPaths: "_creationTime" | "_id";
    indexes: {
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
    };
    searchIndexes: {};
    vectorIndexes: {};
  };
  objectTypes: {
    document: {
      obj: {
        arr: Array<number>;
        bool: boolean;
        data: ArrayBuffer;
        id: Id<"empty">;
        literal: "literal";
        null: null;
        num: number;
        str: string;
      };
      optional?: {
        arr: Array<number>;
        bool: boolean;
        data: ArrayBuffer;
        id: Id<"empty">;
        literal: "literal";
        null: null;
        num: number;
        str: string;
      };
      parent: {
        child: {
          arr: Array<number>;
          bool: boolean;
          data: ArrayBuffer;
          id: Id<"empty">;
          literal: "literal";
          null: null;
          num: number;
          str: string;
        };
      };
      _id: Id<"objectTypes">;
      _creationTime: number;
    };
    fieldPaths:
      | "_creationTime"
      | "_id"
      | "obj"
      | "obj.arr"
      | "obj.bool"
      | "obj.data"
      | "obj.id"
      | "obj.literal"
      | "obj.null"
      | "obj.num"
      | "obj.str"
      | "optional"
      | "optional.arr"
      | "optional.bool"
      | "optional.data"
      | "optional.id"
      | "optional.literal"
      | "optional.null"
      | "optional.num"
      | "optional.str"
      | "parent"
      | "parent.child"
      | "parent.child.arr"
      | "parent.child.bool"
      | "parent.child.data"
      | "parent.child.id"
      | "parent.child.literal"
      | "parent.child.null"
      | "parent.child.num"
      | "parent.child.str";
    indexes: {
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
      child: ["parent.child", "_creationTime"];
      num_bool: ["parent.child.num", "parent.child.bool", "_creationTime"];
      parent: ["parent", "_creationTime"];
      str: ["parent.child.str", "_creationTime"];
    };
    searchIndexes: {};
    vectorIndexes: {};
  };
  primitiveTypes: {
    document: {
      arr: Array<number>;
      bool: boolean;
      data: ArrayBuffer;
      id: Id<"empty">;
      literal: "literal";
      null: null;
      num: number;
      str: string;
      _id: Id<"primitiveTypes">;
      _creationTime: number;
    };
    fieldPaths:
      | "_creationTime"
      | "_id"
      | "arr"
      | "bool"
      | "data"
      | "id"
      | "literal"
      | "null"
      | "num"
      | "str";
    indexes: {
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
      arr: ["arr", "_creationTime"];
      bool: ["bool", "_creationTime"];
      data: ["data", "_creationTime"];
      id: ["id", "_creationTime"];
      literal: ["literal", "_creationTime"];
      null: ["null", "_creationTime"];
      num: ["num", "_creationTime"];
      str: ["str", "_creationTime"];
    };
    searchIndexes: {
      search_str: {
        searchField: "str";
        filterFields:
          | "arr"
          | "bool"
          | "data"
          | "id"
          | "literal"
          | "null"
          | "num";
      };
    };
    vectorIndexes: {
      vector_arr: {
        vectorField: "arr";
        dimensions: number;
        filterFields: "bool" | "data" | "id" | "literal" | "null" | "num";
      };
    };
  };
  recordTypes: {
    document: {
      idKey: Record<Id<"empty">, null | string>;
      strKey: Record<
        string,
        {
          arr: Array<number>;
          bool: boolean;
          data: ArrayBuffer;
          id: Id<"empty">;
          literal: "literal";
          null: null;
          num: number;
          str: string;
        }
      >;
      _id: Id<"recordTypes">;
      _creationTime: number;
    };
    fieldPaths:
      | "_creationTime"
      | "_id"
      | "idKey"
      | `idKey.${string}`
      | "strKey"
      | `strKey.${string}`;
    indexes: {
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
      strKey_idKey: ["strKey", "idKey", "_creationTime"];
    };
    searchIndexes: {
      strKey: {
        searchField: "strKey";
        filterFields: "idKey";
      };
    };
    vectorIndexes: {};
  };
  topLevelUnion: {
    document:
      | { _id: Id<"topLevelUnion">; _creationTime: number }
      | {
          arr: Array<number>;
          bool: boolean;
          data: ArrayBuffer;
          id: Id<"empty">;
          literal: "literal";
          null: null;
          num: number;
          str: string;
          _id: Id<"topLevelUnion">;
          _creationTime: number;
        };
    fieldPaths:
      | "_creationTime"
      | "_id"
      | "arr"
      | "bool"
      | "data"
      | "id"
      | "literal"
      | "null"
      | "num"
      | "str";
    indexes: {
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
      bool: ["bool", "_creationTime"];
    };
    searchIndexes: {};
    vectorIndexes: {};
  };
  unionTypes: {
    document: {
      literals: "literal1" | "literal2";
      optional?:
        | null
        | Id<"empty">
        | string
        | number
        | boolean
        | ArrayBuffer
        | Array<number>
        | {
            arr: Array<number>;
            bool: boolean;
            data: ArrayBuffer;
            id: Id<"empty">;
            literal: "literal";
            null: null;
            num: number;
            str: string;
          }
        | Record<string, null>;
      union:
        | null
        | Id<"empty">
        | string
        | number
        | boolean
        | ArrayBuffer
        | Array<number>
        | {
            arr: Array<number>;
            bool: boolean;
            data: ArrayBuffer;
            id: Id<"empty">;
            literal: "literal";
            null: null;
            num: number;
            str: string;
          }
        | Record<string, null>;
      _id: Id<"unionTypes">;
      _creationTime: number;
    };
    fieldPaths:
      | "_creationTime"
      | "_id"
      | "literals"
      | "optional"
      | `optional.${string}`
      | "optional.arr"
      | "optional.bool"
      | "optional.data"
      | "optional.id"
      | "optional.literal"
      | "optional.null"
      | "optional.num"
      | "optional.str"
      | "union"
      | `union.${string}`
      | "union.arr"
      | "union.bool"
      | "union.data"
      | "union.id"
      | "union.literal"
      | "union.null"
      | "union.num"
      | "union.str";
    indexes: {
      by_id: ["_id"];
      by_creation_time: ["_creationTime"];
      union_optional: ["union", "optional", "_creationTime"];
    };
    searchIndexes: {
      union: {
        searchField: "union";
        filterFields: "optional";
      };
    };
    vectorIndexes: {
      vector_union: {
        vectorField: "union";
        dimensions: number;
        filterFields: "optional";
      };
    };
  };
};

/**
 * The names of all of your Convex tables.
 */
export type TableNames = TableNamesInDataModel<DataModel>;

/**
 * The type of a document stored in Convex.
 *
 * @typeParam TableName - A string literal type of the table name (like "users").
 */
export type Doc<TableName extends TableNames> = DocumentByName<
  DataModel,
  TableName
>;

/**
 * An identifier for a document in Convex.
 *
 * Convex documents are uniquely identified by their `Id`, which is accessible
 * on the `_id` field. To learn more, see [Document IDs](https://docs.convex.dev/using/document-ids).
 *
 * Documents can be loaded using `db.get(id)` in query and mutation functions.
 *
 * IDs are just strings at runtime, but this type can be used to distinguish them from other
 * strings when type checking.
 *
 * @typeParam TableName - A string literal type of the table name (like "users").
 */
export type Id<TableName extends TableNames | SystemTableNames> =
  GenericId<TableName>;
