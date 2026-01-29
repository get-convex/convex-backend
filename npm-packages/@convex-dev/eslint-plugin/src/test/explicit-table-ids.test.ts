import { RuleTester } from "@typescript-eslint/rule-tester";
import { explicitTableIds } from "../lib/explicit-table-ids.js";

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

ruleTester.run("explicit-table-ids", explicitTableIds, {
  valid: [
    // Already migrated get call
    {
      code: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getMessage = query({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            return await ctx.db.get("messages", messageId);
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Already migrated replace call
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const updateMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.replace("messages", messageId, { text: "updated" });
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Already migrated patch call
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const patchMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.patch("messages", messageId, { text: "patched" });
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Already migrated delete call
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const deleteMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.delete("messages", messageId);
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Not a db call
    {
      code: `
        import { query } from "./_generated/server";

        export const list = query({
          args: {},
          handler: async (ctx, args) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Regression test for a false positive in IntDate.get from jsrsasign
    {
      code: `
        declare namespace MyNamespace {
          function get(parameter: number): string;
        }

        async function _ignoreUnrelatedFunctionsFromNamespaces() {
          MyNamespace.get(1);
        }
      `,
      filename: "convex/messages.ts",
    },
    // Ignore methods from lib types
    {
      code: `
        new URL("https://www.convex.dev?test=1").searchParams.get("test");
      `,
      filename: "convex/messages.ts",
    },
    // Ignore .replace on string
    {
      code: `
        console.log("test".replace("test", "test2"));
      `,
      filename: "convex/messages.ts",
    },
    // db.system.get with _scheduled_functions ID
    {
      code: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getScheduledFunction = query({
          args: {},
          handler: async (ctx, args) => {
            const scheduledFunctionId: Id<"_scheduled_functions"> = "123" as any;
            return await ctx.db.system.get(scheduledFunctionId);
          },
        });
      `,
      filename: "convex/scheduled.ts",
    },
    // db.system.get with _storage ID
    {
      code: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getStorage = query({
          args: {},
          handler: async (ctx, args) => {
            const storageId: Id<"_storage"> = "123" as any;
            return await ctx.db.system.get(storageId);
          },
        });
      `,
      filename: "convex/storage.ts",
    },
  ],
  invalid: [
    // Unmigrated get call
    {
      code: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getMessage = query({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            return await ctx.db.get(messageId);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getMessage = query({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            return await ctx.db.get("messages", messageId);
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Unmigrated replace call
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const updateMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.replace(messageId, { text: "updated" });
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const updateMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.replace("messages", messageId, { text: "updated" });
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Unmigrated patch call
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const patchMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.patch(messageId, { text: "patched" });
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const patchMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.patch("messages", messageId, { text: "patched" });
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Unmigrated delete call
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const deleteMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.delete(messageId);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const deleteMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            await ctx.db.delete("messages", messageId);
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Multiple unmigrated calls in one function
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const updateMultiple = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            const userId: Id<"users"> = "456" as any;

            const message = await ctx.db.get(messageId);
            const user = await ctx.db.get(userId);

            await ctx.db.patch(messageId, { author: user?.name });
          },
        });
      `,
      errors: [
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
      ],
      output: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const updateMultiple = mutation({
          args: {},
          handler: async (ctx, args) => {
            const messageId: Id<"messages"> = "123" as any;
            const userId: Id<"users"> = "456" as any;

            const message = await ctx.db.get("messages", messageId);
            const user = await ctx.db.get("users", userId);

            await ctx.db.patch("messages", messageId, { author: user?.name });
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Unmigrated call with different table name
    {
      code: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getUser = query({
          args: {},
          handler: async (ctx, args) => {
            const userId: Id<"users"> = "123" as any;
            return await ctx.db.get(userId);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getUser = query({
          args: {},
          handler: async (ctx, args) => {
            const userId: Id<"users"> = "123" as any;
            return await ctx.db.get("users", userId);
          },
        });
      `,
      filename: "convex/users.ts",
    },
    // Type is `any` - should report error but no auto-fix
    {
      code: `
        import { query } from "./_generated/server";

        export const getMessage = query({
          args: {},
          handler: async (ctx, args) => {
            const id: any = "123";
            return await ctx.db.get(id);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name-no-inference",
        },
      ],
      filename: "convex/messages.ts",
    },
    // Type is `Id<any>` - should report error but no auto-fix
    {
      code: `
        import { query } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const getMessage = query({
          args: {},
          handler: async (ctx, args) => {
            const id: Id<any> = "123" as Id<any>;
            return await ctx.db.get(id);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name-no-inference",
        },
      ],
      filename: "convex/messages.ts",
    },
    // Type is `any` with db.patch - should report error but no auto-fix
    {
      code: `
        import { mutation } from "./_generated/server";

        export const updateMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const id: any = "123";
            await ctx.db.patch(id, { text: "updated" });
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name-no-inference",
        },
      ],
      filename: "convex/messages.ts",
    },
    // Type is `Id<any>` with db.delete - should report error but no auto-fix
    {
      code: `
        import { mutation } from "./_generated/server";
        import { Id } from "./_generated/dataModel";

        export const deleteMessage = mutation({
          args: {},
          handler: async (ctx, args) => {
            const id: Id<any> = "123" as Id<any>;
            await ctx.db.delete(id);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-table-name-no-inference",
        },
      ],
      filename: "convex/messages.ts",
    },
    // DatabaseReader type - should detect and auto-fix
    {
      code: `
        import { Id } from "./_generated/dataModel";
        import { DatabaseReader } from "./_generated/server";

        async function fromDbReader(db: DatabaseReader, id: Id<"documents">) {
          await db.get(id);
        }
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import { Id } from "./_generated/dataModel";
        import { DatabaseReader } from "./_generated/server";

        async function fromDbReader(db: DatabaseReader, id: Id<"documents">) {
          await db.get("documents", id);
        }
      `,
      filename: "convex/helpers.ts",
    },
    // GenericDatabaseReader type - should detect and auto-fix
    {
      code: `
        import {
          GenericDatabaseReader,
          GenericDataModel,
        } from "convex/server";
        import { Id } from "./_generated/dataModel";

        async function fromGenericDbReader(
          db: GenericDatabaseReader<GenericDataModel>,
          id: Id<"documents">,
        ) {
          await db.get(id);
        }
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import {
          GenericDatabaseReader,
          GenericDataModel,
        } from "convex/server";
        import { Id } from "./_generated/dataModel";

        async function fromGenericDbReader(
          db: GenericDatabaseReader<GenericDataModel>,
          id: Id<"documents">,
        ) {
          await db.get("documents", id);
        }
      `,
      filename: "convex/helpers.ts",
    },
    // GenericDatabaseReader with extends - should detect and auto-fix
    {
      code: `
        import {
          GenericDatabaseReader,
          GenericDataModel,
        } from "convex/server";
        import { Id } from "./_generated/dataModel";

        async function fromGenericDbReaderExtends<
          SomeDataModel extends GenericDataModel,
        >(db: GenericDatabaseReader<SomeDataModel>, id: Id<"documents">) {
          await db.get(id);
        }
      `,
      errors: [
        {
          messageId: "missing-table-name",
        },
      ],
      output: `
        import {
          GenericDatabaseReader,
          GenericDataModel,
        } from "convex/server";
        import { Id } from "./_generated/dataModel";

        async function fromGenericDbReaderExtends<
          SomeDataModel extends GenericDataModel,
        >(db: GenericDatabaseReader<SomeDataModel>, id: Id<"documents">) {
          await db.get("documents", id);
        }
      `,
      filename: "convex/helpers.ts",
    },
    // DatabaseWriter type - should detect and auto-fix all methods
    {
      code: `
        import { Id } from "./_generated/dataModel";
        import { DatabaseWriter } from "./_generated/server";

        async function fromDbWriter(db: DatabaseWriter, id: Id<"documents">) {
          await db.get(id);
          await db.replace(id, { name: "test2" });
          await db.patch(id, { name: "test3" });
          await db.delete(id);
        }
      `,
      errors: [
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
      ],
      output: `
        import { Id } from "./_generated/dataModel";
        import { DatabaseWriter } from "./_generated/server";

        async function fromDbWriter(db: DatabaseWriter, id: Id<"documents">) {
          await db.get("documents", id);
          await db.replace("documents", id, { name: "test2" });
          await db.patch("documents", id, { name: "test3" });
          await db.delete("documents", id);
        }
      `,
      filename: "convex/helpers.ts",
    },
    // GenericDatabaseWriter type - should detect and auto-fix all methods
    {
      code: `
        import {
          GenericDatabaseWriter,
          GenericDataModel,
        } from "convex/server";
        import { Id } from "./_generated/dataModel";

        async function fromGenericDbWriter(
          db: GenericDatabaseWriter<GenericDataModel>,
          id: Id<"documents">,
        ) {
          await db.get(id);
          await db.replace(id, { name: "test2" });
          await db.patch(id, { name: "test3" });
          await db.delete(id);
        }
      `,
      errors: [
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
        { messageId: "missing-table-name" },
      ],
      output: `
        import {
          GenericDatabaseWriter,
          GenericDataModel,
        } from "convex/server";
        import { Id } from "./_generated/dataModel";

        async function fromGenericDbWriter(
          db: GenericDatabaseWriter<GenericDataModel>,
          id: Id<"documents">,
        ) {
          await db.get("documents", id);
          await db.replace("documents", id, { name: "test2" });
          await db.patch("documents", id, { name: "test3" });
          await db.delete("documents", id);
        }
      `,
      filename: "convex/helpers.ts",
    },
  ],
});
