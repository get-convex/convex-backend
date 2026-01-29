import { RuleTester } from "@typescript-eslint/rule-tester";
import { noCollectInQuery } from "../lib/no-collect-in-query.js";

const dbQuerySetup = `
  import {
    DataModelFromSchemaDefinition,
    defineSchema,
    defineTable,
    GenericDatabaseReader,
    GenericDatabaseReaderWithTable,
  } from "convex/server";
  import { v } from "convex/values";

  const _schema = defineSchema({
    messages: defineTable({
      body: v.string(),
      channel: v.string(),
    })
      .index("by_body", ["body"])
      .index("by_channel_body", ["channel", "body"])
      .searchIndex("search_body", {
        searchField: "body",
        filterFields: ["channel"],
      }),
  });

  type DataModel = DataModelFromSchemaDefinition<typeof _schema>;

  declare const db: GenericDatabaseReader<DataModel>;
  declare const dbWithTable: GenericDatabaseReaderWithTable<DataModel>;
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

ruleTester.run("no-collect-in-query", noCollectInQuery, {
  valid: [
    // Calling `collect()` on `any` should not warn (no type info).
    {
      code: `
        const q: any = null;
        q.collect();
      `,
      filename: "shared.ts",
    },
    // Calling `collect()` on some other type should not warn.
    {
      code: `
        type NotAQuery = { collect(): Promise<number> };
        declare const q: NotAQuery;
        void q.collect();
      `,
      filename: "shared.ts",
    },
    // Query consumer methods other than `collect()` should not warn.
    {
      code: `
        ${dbQuerySetup}
        async function f() {
          await db.query("messages").take(10);
          await db.query("messages").paginate({ numItems: 10, cursor: null });
          await db.query("messages").first();
          await db.query("messages").unique();
          await db.query("messages").count();
        }
      `,
      filename: "convex/messages.ts",
    },
    // Even if types match, we intentionally ignore computed property access.
    {
      code: `
        import { OrderedQuery } from "convex/server";
        declare const q: OrderedQuery<any>;
        void q["collect"]();
      `,
      filename: "convex/messages.ts",
    },
    // False positives: local "OrderedQuery" that isn't Convex's OrderedQuery.
    {
      code: `
        interface OrderedQuery<T> {
          collect(): Promise<T[]>;
          take(n: number): Promise<T[]>;
        }
        declare const q: OrderedQuery<number>;
        void q.collect();
        void q.take(10);
      `,
      filename: "shared.ts",
    },
    // False positives: "Convex-looking" db/query/withIndex/collect, but not typed as Convex.
    {
      code: `
        type NotConvexQuery = {
          withIndex(name: string): NotConvexQuery;
          order(order: "asc" | "desc"): NotConvexQuery;
          filter(cb: (q: unknown) => boolean): NotConvexQuery;
          collect(): Promise<number[]>;
        };
        type NotConvexDb = { query(table: string): NotConvexQuery };
        declare const db: NotConvexDb;
        void db.query("messages").withIndex("by_body").order("asc").filter(() => true).collect();
      `,
      filename: "convex/messages.ts",
    },
  ],
  invalid: [
    // Shared code that calls `collect()` on a Convex query should still warn.
    {
      code: `
        import { OrderedQuery } from "convex/server";

        export async function collectAll(q: OrderedQuery<any>) {
          return await q.collect();
        }
      `,
      filename: "convex/helpers.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        import { OrderedQuery } from "convex/server";

        export async function collectAll(q: OrderedQuery<any>) {
          return await q.take();
        }
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        import { OrderedQuery } from "convex/server";

        export async function collectAll(q: OrderedQuery<any>) {
          return await q.paginate();
        }
      `,
            },
          ],
        },
      ],
    },
    // Direct `OrderedQuery.collect()` should warn.
    {
      code: `
        import { OrderedQuery } from "convex/server";

        declare const q: OrderedQuery<any>;
        void q.collect();
      `,
      filename: "shared.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        import { OrderedQuery } from "convex/server";

        declare const q: OrderedQuery<any>;
        void q.take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        import { OrderedQuery } from "convex/server";

        declare const q: OrderedQuery<any>;
        void q.paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).collect()` should warn (common user syntax).
    {
      code: `
        ${dbQuerySetup}
        void db.query("messages").collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db.query("messages").take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db.query("messages").paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).withIndex(...).collect()` should warn.
    {
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_channel_body", (q) => q.eq("channel", "general"))
          .collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_channel_body", (q) => q.eq("channel", "general"))
          .take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_channel_body", (q) => q.eq("channel", "general"))
          .paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).withIndex(...).order(...).collect()` should warn (OrderedQuery chain).
    {
      code: `
        ${dbQuerySetup}
        void db.query("messages").withIndex("by_body").order("desc").collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db.query("messages").withIndex("by_body").order("desc").take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db.query("messages").withIndex("by_body").order("desc").paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).withSearchIndex(...).collect()` should warn.
    {
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withSearchIndex("search_body", (q) => q.search("body", "hello"))
          .collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withSearchIndex("search_body", (q) => q.search("body", "hello"))
          .take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withSearchIndex("search_body", (q) => q.search("body", "hello"))
          .paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).fullTableScan().collect()` should warn.
    {
      code: `
        ${dbQuerySetup}
        void db.query("messages").fullTableScan().collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db.query("messages").fullTableScan().take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db.query("messages").fullTableScan().paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).order(...).collect()` should warn.
    {
      code: `
        ${dbQuerySetup}
        void db.query("messages").order("desc").collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db.query("messages").order("desc").take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db.query("messages").order("desc").paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.query(...).filter(...).collect()` should warn.
    {
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .filter((q) => q.eq(q.field("channel"), "general"))
          .collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .filter((q) => q.eq(q.field("channel"), "general"))
          .take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .filter((q) => q.eq(q.field("channel"), "general"))
          .paginate();
      `,
            },
          ],
        },
      ],
    },
    // Combine several query methods: `withIndex(...).order(...).filter(...).collect()`.
    {
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_channel_body", (q) => q.eq("channel", "general"))
          .order("asc")
          .filter((q) => q.neq(q.field("body"), "ignore"))
          .collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_channel_body", (q) => q.eq("channel", "general"))
          .order("asc")
          .filter((q) => q.neq(q.field("body"), "ignore"))
          .take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withIndex("by_channel_body", (q) => q.eq("channel", "general"))
          .order("asc")
          .filter((q) => q.neq(q.field("body"), "ignore"))
          .paginate();
      `,
            },
          ],
        },
      ],
    },
    // `withSearchIndex(...).filter(...).collect()` should warn.
    {
      code: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withSearchIndex("search_body", (q) =>
            q.search("body", "hello").eq("channel", "general"),
          )
          .filter((q) => q.eq(q.field("channel"), "general"))
          .collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withSearchIndex("search_body", (q) =>
            q.search("body", "hello").eq("channel", "general"),
          )
          .filter((q) => q.eq(q.field("channel"), "general"))
          .take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void db
          .query("messages")
          .withSearchIndex("search_body", (q) =>
            q.search("body", "hello").eq("channel", "general"),
          )
          .filter((q) => q.eq(q.field("channel"), "general"))
          .paginate();
      `,
            },
          ],
        },
      ],
    },
    // `db.table("messages").query().collect()` should warn (scoped table reader API).
    {
      code: `
        ${dbQuerySetup}
        void dbWithTable.table("messages").query().collect();
      `,
      filename: "convex/messages.ts",
      errors: [
        {
          messageId: "no-collect-in-query",
          suggestions: [
            {
              messageId: "replace-with-take",
              output: `
        ${dbQuerySetup}
        void dbWithTable.table("messages").query().take();
      `,
            },
            {
              messageId: "replace-with-paginate",
              output: `
        ${dbQuerySetup}
        void dbWithTable.table("messages").query().paginate();
      `,
            },
          ],
        },
      ],
    },
  ],
});
