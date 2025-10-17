import { RuleTester } from "@typescript-eslint/rule-tester";
import { noOldRegisteredFunctionSyntax } from "../lib/no-old-registered-function-syntax.js";

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

ruleTester.run(
  "no-old-registered-function-syntax",
  noOldRegisteredFunctionSyntax,
  {
    valid: [
      // Valid object syntax for query
      {
        code: `
        import { query } from "./_generated/server";
        export const list = query({
          handler: async (ctx) => {
            return await ctx.db.query("messages").collect();
          },
        });
      `,
        filename: "convex/messages.ts",
      },
      // Valid object syntax for mutation with args
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
    ],
    invalid: [
      // Invalid function syntax for query - no second parameter, so adds empty args
      {
        code: `
        import { query } from "./_generated/server";
        export const list = query(async (ctx) => {
          return await ctx.db.query("messages").collect();
        });
      `,
        errors: [{ messageId: "use-object-syntax" }],
        filename: "convex/messages.ts",
        output: `
        import { query } from "./_generated/server";
        export const list = query({
  args: {},
  handler: async (ctx) => {
          return await ctx.db.query("messages").collect();
        }
});
      `,
      },
      // Invalid function syntax for mutation - has second parameter, so doesn't add args
      {
        code: `
        import { mutation } from "./_generated/server";
        export const send = mutation(async (ctx, { body, author }) => {
          const message = { body, author };
          await ctx.db.insert("messages", message);
        });
      `,
        errors: [{ messageId: "use-object-syntax" }],
        filename: "convex/messages.ts",
        output: `
        import { mutation } from "./_generated/server";
        export const send = mutation({
  handler: async (ctx, { body, author }) => {
          const message = { body, author };
          await ctx.db.insert("messages", message);
        }
});
      `,
      },
      // No second parameter, so adds empty args
      {
        code: `
        import { action } from "./_generated/server";

        export const nop = action(async () => {});

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
        errors: [{ messageId: "use-object-syntax" }],
        filename: "convex/helpers.ts",
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
      },
      // Action with second parameter - doesn't add args
      {
        code: `
        import { action } from "./_generated/server";
        export const tac = action(async ({ runMutation }, { author }) => {
          await runMutation(api.sendMessage.default, {
            body: "tac",
            author,
          });
        });
      `,
        errors: [{ messageId: "use-object-syntax" }],
        filename: "convex/actions.ts",
        output: `
        import { action } from "./_generated/server";
        export const tac = action({
  handler: async ({ runMutation }, { author }) => {
          await runMutation(api.sendMessage.default, {
            body: "tac",
            author,
          });
        }
});
      `,
      },
    ],
  },
);
