import { describe, expect, test } from "vitest";
import {
  generateColumnRenameScaffold,
  generateTableRenameScaffold,
} from "./renameScaffold";

const columnArgs = {
  tableName: "posts",
  oldColumnName: "title",
  newColumnName: "heading",
};

const tableArgs = {
  oldTableName: "posts",
  newTableName: "articles",
};

describe("rename scaffold generation", () => {
  test("generates a column rename scaffold", () => {
    const scaffold = generateColumnRenameScaffold({
      ...columnArgs,
      backfill: true,
    });

    expect(scaffold.mutationCode).toContain("renamePostsTitleToHeading");
    expect(scaffold.mutationCode).toContain(
      "await ctx.db.patch(doc._id, { heading: value, title: undefined });",
    );
    expect(scaffold.schemaDiff).toBe(`// convex/schema.ts
posts: defineTable({
-  title: v.string(),
+  heading: v.string(),
  // ...
}),`);
  });

  test("generates a table rename scaffold", () => {
    const scaffold = generateTableRenameScaffold({
      ...tableArgs,
      backfill: true,
    });

    expect(scaffold.mutationCode).toContain("renamePostsToArticles");
    expect(scaffold.mutationCode).toContain(
      'await ctx.db.insert("articles", value);',
    );
    expect(scaffold.mutationCode).toContain("await ctx.db.delete(doc._id);");
    expect(scaffold.schemaDiff).toBe(`// convex/schema.ts
-posts: defineTable({
+articles: defineTable({
  // ...
}),`);
  });

  test("omits mutation code when backfill is off", () => {
    const scaffold = generateColumnRenameScaffold({
      ...columnArgs,
      backfill: false,
    });

    expect(scaffold.mutationCode).toBe("");
    expect(scaffold.schemaDiff).toContain("+  heading: v.string(),");
  });

  test("escapes fields and handles table underscores", () => {
    const columnScaffold = generateColumnRenameScaffold({
      tableName: "posts",
      oldColumnName: "old-title",
      newColumnName: "newTitle",
      backfill: true,
    });
    const tableScaffold = generateTableRenameScaffold({
      oldTableName: "user_profiles",
      newTableName: "member_profiles",
      backfill: true,
    });

    expect(columnScaffold.mutationCode).toContain(
      'const value = doc["old-title"];',
    );
    expect(columnScaffold.mutationCode).toContain(
      'await ctx.db.patch(doc._id, { newTitle: value, "old-title": undefined });',
    );
    expect(columnScaffold.schemaDiff).toContain('-  "old-title": v.string(),');
    expect(tableScaffold.mutationCode).toContain(
      "renameUserProfilesToMemberProfiles",
    );
    expect(tableScaffold.schemaDiff).toContain("-user_profiles: defineTable({");
  });
});
