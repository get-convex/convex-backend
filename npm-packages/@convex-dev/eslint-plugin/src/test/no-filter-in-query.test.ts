import { RuleTester } from "@typescript-eslint/rule-tester";
import { noFilterInQuery } from "../lib/no-filter-in-query.js";

const dbQuerySetup = `
  import {
    DataModelFromSchemaDefinition,
    defineSchema,
    defineTable,
    GenericDatabaseReader,
  } from "convex/server";
  import { v } from "convex/values";

  const _schema = defineSchema({
    messages: defineTable({
      author: v.string(),
      body: v.string(),
      isDeleted: v.boolean(),
    })
      .index("by_author", ["author"])
      .index("by_body", ["body"]),
  });

  type DataModel = DataModelFromSchemaDefinition<typeof _schema>;
  declare const db: GenericDatabaseReader<DataModel>;
`;

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      ecmaVersion: 2020,
      sourceType: "module",
      projectService: {
        allowDefaultProject: ["*.ts*", "convex/*.ts*"],
      },
    },
  },
});

ruleTester.run("no-filter-in-query", noFilterInQuery, {
  valid: [
    {
      name: "Array `.filter()` after `.collect()` is outside the scope of this rule",
      code: `
        import { query } from "./_generated/server";

        export const list = query({
          args: {},
          handler: async (ctx) => {
            const results = await ctx.db.query("messages").collect();
            return results.filter((m) => m.author === "alice");
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    {
      name: "Unrelated `.filter()` usage is fine",
      code: `
        const xs = [1, 2, 3];
        const ys = xs.filter((x) => x > 1);
        console.log(ys);
      `,
      filename: "convex/helpers.ts",
    },
    {
      name: "Any-typed query receiver `.filter()` should not be flagged",
      code: `
        import { query } from "./_generated/server";

        export const list = query({
          args: {},
          handler: async (ctx) => {
            const maybeAny: any = ctx.db.query("messages");
            return await maybeAny.filter((x: any) => x?.ok).collect();
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    {
      name: "Convex-looking `db.query().filter()` chain that is not Convex should not be flagged",
      code: `
        type NotConvexQuery = {
          withIndex(name: string): NotConvexQuery;
          order(order: "asc" | "desc"): NotConvexQuery;
          filter(cb: (q: unknown) => boolean): NotConvexQuery;
          collect(): Promise<number[]>;
        };
        type NotConvexDb = { query(table: string): NotConvexQuery };
        declare const db: NotConvexDb;

        void db
          .query("messages")
          .withIndex("by_body")
          .order("asc")
          .filter(() => true)
          .collect();
      `,
      filename: "convex/messages.ts",
    },
  ],
  invalid: [
    {
      name: "Direct query `.filter()` call",
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .filter((q) => q.eq(q.field("author"), "alice"))
          .collect();
      `,
      filename: "convex/messages.ts",
      errors: [{ messageId: "no-filter-in-query" }],
    },
    {
      name: "Chained query `.filter()` call (withIndex)",
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_author", (q) => q.eq("author", "alice"))
          .filter((q) => q.eq(q.field("isDeleted"), false))
          .take(10);
      `,
      filename: "convex/messages.ts",
      errors: [{ messageId: "no-filter-in-query" }],
    },
    {
      name: "Query stored in a variable then `.filter()`",
      code: `
        import { query } from "./_generated/server";

        export const list = query({
          args: {},
          handler: async (ctx) => {
            const q = ctx.db.query("messages");
            return await q
              .filter((q) => q.eq(q.field("author"), "alice"))
              .collect();
          },
        });
      `,
      filename: "convex/messages.ts",
      errors: [{ messageId: "no-filter-in-query" }],
    },
    {
      name: "Destructured db handler `.filter()` call",
      code: `
        ${dbQuerySetup}
        async function run({ db }: { db: GenericDatabaseReader<DataModel> }) {
          return await db
            .query("messages")
            .filter((q) => q.eq(q.field("author"), "alice"))
            .collect();
        }
      `,
      filename: "convex/messages.ts",
      errors: [{ messageId: "no-filter-in-query" }],
    },
  ],
});
