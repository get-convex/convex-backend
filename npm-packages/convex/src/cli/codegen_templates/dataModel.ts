import { GeneratedJsWithTypes, header } from "./common.js";

const dataModelDTS = `
  ${header("Generated data model types.")}
  import type { DataModelFromSchemaDefinition, DocumentByName, TableNamesInDataModel, SystemTableNames } from "convex/server";
  import type { GenericId } from "convex/values";
  import schema from "../schema.js";
  
  /**
   * The names of all of your Convex tables.
   */
  export type TableNames = TableNamesInDataModel<DataModel>;
  
  /**
   * The type of a document stored in Convex.
   * 
   * @typeParam TableName - A string literal type of the table name (like "users").
   */
  export type Doc<TableName extends TableNames> = DocumentByName<DataModel, TableName>;

  /**
   * An identifier for a document in Convex.
   *
   * Convex documents are uniquely identified by their \`Id\`, which is accessible
   * on the \`_id\` field. To learn more, see [Document IDs](https://docs.convex.dev/using/document-ids).
   *
   * Documents can be loaded using \`db.get(id)\` in query and mutation functions.
   * 
   * IDs are just strings at runtime, but this type can be used to distinguish them from other
   * strings when type checking.
   * 
   * @typeParam TableName - A string literal type of the table name (like "users").
   */
  export type Id<TableName extends TableNames | SystemTableNames> = GenericId<TableName>;

  /**
   * A type describing your Convex data model.
   * 
   * This type includes information about what tables you have, the type of
   * documents stored in those tables, and the indexes defined on them.
   * 
   * This type is used to parameterize methods like \`queryGeneric\` and 
   * \`mutationGeneric\` to make them type-safe. 
   */
  export type DataModel = DataModelFromSchemaDefinition<typeof schema>;
  `;

export const dataModel: GeneratedJsWithTypes = {
  DTS: dataModelDTS,
};

const dataModelWithoutSchemaDTS = `
  ${header("Generated data model types.")}
  import { AnyDataModel } from "convex/server";
  import type { GenericId } from "convex/values";

  /**
   * No \`schema.ts\` file found!
   * 
   * This generated code has permissive types like \`Doc = any\` because
   * Convex doesn't know your schema. If you'd like more type safety, see
   * https://docs.convex.dev/using/schemas for instructions on how to add a
   * schema file.
   * 
   * After you change a schema, rerun codegen with \`npx convex dev\`.
   */
  
  /**
   * The names of all of your Convex tables.
   */
  export type TableNames = string;
    
  /**
   * The type of a document stored in Convex.
   */
  export type Doc = any;

  /**
   * An identifier for a document in Convex.
   *
   * Convex documents are uniquely identified by their \`Id\`, which is accessible
   * on the \`_id\` field. To learn more, see [Document IDs](https://docs.convex.dev/using/document-ids).
   *
   * Documents can be loaded using \`db.get(id)\` in query and mutation functions.
   *
   * IDs are just strings at runtime, but this type can be used to distinguish them from other
   * strings when type checking.
   */
  export type Id<TableName extends TableNames = TableNames> = GenericId<TableName>;

  /**
   * A type describing your Convex data model.
   * 
   * This type includes information about what tables you have, the type of
   * documents stored in those tables, and the indexes defined on them.
   * 
   * This type is used to parameterize methods like \`queryGeneric\` and 
   * \`mutationGeneric\` to make them type-safe. 
   */
  export type DataModel = AnyDataModel;`;

/**
 * Codegen used when there isn't a `schema.ts` file yet.
 *
 * Make sure that the exports of this are the same as the real version above.
 */
export const dataModelWithoutSchema: GeneratedJsWithTypes = {
  DTS: dataModelWithoutSchemaDTS,
};
