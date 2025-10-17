import { RuleTester } from "@typescript-eslint/rule-tester";
import { requireArgsValidator } from "../lib/require-args-validator.js";

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

ruleTester.run("require-args-validator", requireArgsValidator, {
  valid: [
    {
      name: "New syntax: with args",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query({
          args: { limit: v.number() },
          handler: async (ctx, { limit }) => {
            return await ctx.db.query("messages").take(limit);
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    {
      name: "New syntax: explicit empty args",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query({
          args: {},
          handler: async (ctx, {}) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    {
      name: "Old syntax: no args + ignoreUnusedArguments",
      code: `
            import { v } from "convex/values";
            import { query } from "./_generated/server";
            export const list = query(async (ctx) => {
              return await ctx.db.query("messages").collect();
            });
          `,
      filename: "convex/messages.ts",
      options: [
        {
          ignoreUnusedArguments: true,
        },
      ],
    },
    {
      name: "Old syntax: explicit empty args + ignoreUnusedArguments",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query(async (ctx, {}) => {
          return await ctx.db.query("messages").collect();
        });
      `,
      filename: "convex/messages.ts",
      options: [
        {
          ignoreUnusedArguments: true,
        },
      ],
    },
    {
      name: "New syntax: explicit args that use v.any()",
      code: `
        import { v } from "convex/values";
        import { action } from "./_generated/server";
        export const fetchData = action({
          args: { url: v.string(), params: v.any() },
          handler: async (ctx, { url, params }) => {
            // ... action logic
          },
        });
      `,
      filename: "convex/actions.ts",
    },
  ],
  invalid: [
    {
      name: "New syntax: missing `args` but using arguments - should NOT be autofixable",
      code: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
          handler: async (ctx, { body, author }) => {
            const message = { body, author };
            await ctx.db.insert("messages", message);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-args",
        },
      ],
      filename: "convex/messages.ts",
    },

    {
      name: "New syntax: missing `args` but using typed arguments - should NOT be autofixable",
      code: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
          handler: async (ctx, { body, author }: { body: string, author: string }) => {
            const message = { body, author };
            await ctx.db.insert("messages", message);
          },
        });
      `,
      errors: [
        {
          messageId: "missing-args",
        },
      ],
      filename: "convex/messages.ts",
    },

    {
      name: "New syntax: missing `args` but not using arguments, without ignoreUnusedArguments",
      code: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
          handler: async (ctx) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
      errors: [
        {
          messageId: "missing-empty-args",
        },
      ],
      filename: "convex/messages.ts",
      options: [
        {
          ignoreUnusedArguments: false,
        },
      ],
      output: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
  args: {},
          handler: async (ctx) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
    },

    {
      name: "New syntax: missing `args` + explicit empty arguments, without ignoreUnusedArguments",
      code: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
          handler: async (ctx, {}) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
      errors: [
        {
          messageId: "missing-empty-args",
        },
      ],
      filename: "convex/messages.ts",
      options: [
        {
          ignoreUnusedArguments: false,
        },
      ],
      output: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
  args: {},
          handler: async (ctx, {}) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
    },

    {
      name: "Old syntax: with empty args without ignoreUnusedArguments",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query(async (ctx, {}) => {
          return await ctx.db.query("messages").collect();
        });
      `,
      errors: [
        {
          messageId: "missing-empty-args",
        },
      ],
      filename: "convex/messages.ts",
      options: [
        {
          ignoreUnusedArguments: false,
        },
      ],
      output: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query({
  args: {},
  handler: async (ctx, {}) => {
          return await ctx.db.query("messages").collect();
        }
});
      `,
    },
    {
      name: "Old syntax: with no args without ignoreUnusedArguments",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query(async (ctx) => {
          return await ctx.db.query("messages").collect();
        });
      `,
      errors: [
        {
          messageId: "missing-empty-args",
        },
      ],
      filename: "convex/messages.ts",
      options: [
        {
          ignoreUnusedArguments: false,
        },
      ],
      output: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query({
  args: {},
  handler: async (ctx) => {
          return await ctx.db.query("messages").collect();
        }
});
      `,
    },
    {
      name: "Old syntax: with args that are used (not autofixable)",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query(async (ctx, { message }) => {
          console.log(message);
        });
      `,
      errors: [
        {
          messageId: "missing-args",
        },
      ],
      filename: "convex/messages.ts",
    },
    {
      name: "Old syntax: with typed args that are used (not autofixable)",
      code: `
        import { v } from "convex/values";
        import { query } from "./_generated/server";
        export const list = query(async (ctx, { message }: { message: string }) => {
          console.log(message);
        });
      `,
      errors: [
        {
          messageId: "missing-args",
        },
      ],
      filename: "convex/messages.ts",
    },
  ],
});
