import { RuleTester } from "@typescript-eslint/rule-tester";
import tseslint from "typescript-eslint";
import { noUnexportedConvexFunction } from "./no-unexported-convex-function.js";

const ruleTester = new RuleTester({
  languageOptions: {
    parser: tseslint.parser,
    parserOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
    },
  },
});

const filename = "convex/messages.ts";

ruleTester.run(
  "no-unexported-convex-function",
  noUnexportedConvexFunction,
  {
    valid: [
      {
        filename,
        code: 'export const send = mutation({ handler: async () => null });',
      },
      {
        filename,
        code: [
          'const send = mutation({ handler: async () => null });',
          "export { send };",
        ].join("\n"),
      },
      {
        filename,
        code: 'const send = authMutation({ handler: async () => null });',
      },
      {
        filename: "convex\\_generated\\server.ts",
        code: 'const send = mutation({ handler: async () => null });',
      },
    ],
    invalid: [
      {
        filename,
        code: 'const send = mutation({ handler: async () => null });',
        errors: [
          {
            messageId: "no-unexported-convex-function",
            data: { name: "send", registrar: "mutation" },
          },
        ],
      },
      {
        filename,
        code: 'const send = authMutation({ handler: async () => null });',
        options: [{ additionalRegistrars: ["authMutation"] }],
        errors: [
          {
            messageId: "no-unexported-convex-function",
            data: { name: "send", registrar: "authMutation" },
          },
        ],
      },
    ],
  },
);
