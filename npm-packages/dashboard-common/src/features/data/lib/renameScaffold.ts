export function generateColumnRenameScaffold({
  tableName,
  oldColumnName,
  newColumnName,
  backfill,
}: {
  tableName: string;
  oldColumnName: string;
  newColumnName: string;
  backfill: boolean;
}): { mutationCode: string; schemaDiff: string } {
  const schemaDiff = `// convex/schema.ts
${propertyKey(tableName)}: defineTable({
-  ${propertyKey(oldColumnName)}: v.string(),
+  ${propertyKey(newColumnName)}: v.string(),
  // ...
}),`;

  if (!backfill) {
    return { mutationCode: "", schemaDiff };
  }

  return {
    mutationCode: `// convex/migrations.ts
import { internalMutation } from "./_generated/server";

export const rename${pascalCase(tableName)}${pascalCase(oldColumnName)}To${pascalCase(newColumnName)} = internalMutation({
  args: {},
  handler: async (ctx) => {
    const docs = await ctx.db.query(${JSON.stringify(tableName)}).collect();
    for (const doc of docs) {
      const value = ${fieldAccess("doc", oldColumnName)};
      if (value !== undefined) {
        await ctx.db.patch(doc._id, { ${propertyAssignment(newColumnName, "value")}, ${propertyAssignment(oldColumnName, "undefined")} });
      }
    }
  },
});`,
    schemaDiff,
  };
}

export function generateTableRenameScaffold({
  oldTableName,
  newTableName,
  backfill,
}: {
  oldTableName: string;
  newTableName: string;
  backfill: boolean;
}): { mutationCode: string; schemaDiff: string } {
  const schemaDiff = `// convex/schema.ts
-${propertyKey(oldTableName)}: defineTable({
+${propertyKey(newTableName)}: defineTable({
  // ...
}),`;

  if (!backfill) {
    return { mutationCode: "", schemaDiff };
  }

  return {
    mutationCode: `// convex/migrations.ts
import { internalMutation } from "./_generated/server";

export const rename${pascalCase(oldTableName)}To${pascalCase(newTableName)} = internalMutation({
  args: {},
  handler: async (ctx) => {
    const docs = await ctx.db.query(${JSON.stringify(oldTableName)}).collect();
    for (const doc of docs) {
      const value = { ...doc };
      delete value._id;
      delete value._creationTime;
      await ctx.db.insert(${JSON.stringify(newTableName)}, value);
      await ctx.db.delete(doc._id);
    }
  },
});`,
    schemaDiff,
  };
}

function propertyKey(name: string) {
  return isIdentifier(name) ? name : JSON.stringify(name);
}

function propertyAssignment(name: string, value: string) {
  return isIdentifier(name)
    ? `${name}: ${value}`
    : `${JSON.stringify(name)}: ${value}`;
}

function fieldAccess(objectName: string, fieldName: string) {
  return isIdentifier(fieldName)
    ? `${objectName}.${fieldName}`
    : `${objectName}[${JSON.stringify(fieldName)}]`;
}

function isIdentifier(name: string) {
  return /^[A-Za-z_$][\w$]*$/.test(name);
}

function pascalCase(name: string) {
  const words = name.match(/[A-Za-z0-9]+/g) ?? ["Field"];
  return words
    .map((word) => `${word[0].toUpperCase()}${word.slice(1)}`)
    .join("");
}
