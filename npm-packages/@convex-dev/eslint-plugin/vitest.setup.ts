import * as vitest from "vitest";
import { RuleTester } from "@typescript-eslint/rule-tester";

// Configure RuleTester to use Vitest
RuleTester.afterAll = vitest.afterAll;

// If you're not using Vitest with globals: true
RuleTester.it = vitest.it;
RuleTester.itOnly = vitest.it.only;
RuleTester.describe = vitest.describe;
