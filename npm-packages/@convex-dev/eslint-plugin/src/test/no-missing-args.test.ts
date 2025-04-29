import { RuleTester } from "@typescript-eslint/rule-tester";
import { noMissingArgs } from "../lib/noMissingArgs.js";

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

ruleTester.run("no-missing-args-validator", noMissingArgs, {
  valid: [
    // Query with args
    {
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
    // Query with empty args object
    {
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
    // Mutation with args
    {
      code: `
        import { v } from "convex/values";
        import { mutation } from "./_generated/server";
        export const send = mutation({
          args: { body: v.string(), author: v.string() },
          handler: async (ctx, { body, author }) => {
            const message = { body, author };
            await ctx.db.insert("messages", message);
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Action with args using v.any()
    {
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
    // Missing args in query with no second parameter - should be autofixable
    {
      code: `
        import { query } from "./_generated/server";
        export const list = query({
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
      output: `
        import { query } from "./_generated/server";
        export const list = query({
  args: {},

          handler: async (ctx) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
      filename: "convex/messages.ts",
    },
    // Missing args in mutation with second parameter - should NOT be autofixable
    {
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
    // Missing args in action with second parameter - should NOT be autofixable
    {
      code: `
        import { action } from "./_generated/server";
        export const fetchData = action({
          handler: async (ctx, { url }) => {
            // ... action logic
          },
        });
      `,
      errors: [
        {
          messageId: "missing-args",
        },
      ],
      filename: "convex/actions.ts",
    },
    // Missing args in simple function with no second parameter - should be autofixable
    {
      code: `
        import { action } from "./_generated/server";

        export const nop = action({
          handler: async () => {}
        });

        /**
         * This function is not source-mappable by our current analyze because this file
         * include a helper function used in another entry point. That makes the bundler
         * stick both of these functions in deps/ file and makes this file just a
         * re-export of that nop function.
         */

        export function helper(a: number, b: number): number {
          return a + b;
        }
      `,
      errors: [
        {
          messageId: "missing-empty-args",
        },
      ],
      output: `
        import { action } from "./_generated/server";

        export const nop = action({
  args: {},

          handler: async () => {}
        });

        /**
         * This function is not source-mappable by our current analyze because this file
         * include a helper function used in another entry point. That makes the bundler
         * stick both of these functions in deps/ file and makes this file just a
         * re-export of that nop function.
         */

        export function helper(a: number, b: number): number {
          return a + b;
        }
      `,
      filename: "convex/helpers.ts",
    },
    // Additional test for a function with handler and only ctx parameter
    {
      code: `
        import { query } from "./_generated/server";
        export const listMessages = query({
          handler: async (ctx) => {
            return await ctx.db.query("messages").collect();
          }
        });
      `,
      errors: [
        {
          messageId: "missing-empty-args",
        },
      ],
      output: `
        import { query } from "./_generated/server";
        export const listMessages = query({
  args: {},

          handler: async (ctx) => {
            return await ctx.db.query("messages").collect();
          }
        });
      `,
      filename: "convex/messages.ts",
    },
  ],
});
