import { expect, test } from "vitest";

import { defineApp } from "./index.js";

test("app.use throws on empty component name", () => {
  const app = defineApp() as any;

  const importedComponentDefinition = {
    componentDefinitionPath: "components/workflow",
    defaultName: "workflow",
  } as any;

  expect(() => app.use(importedComponentDefinition, { name: "" })).toThrow(
    /component name cannot be empty/i,
  );
});
