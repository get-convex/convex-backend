import { screen, render } from "@testing-library/react";
import { useRouter } from "next/router";
import { Shape } from "shapes";
import { useQuery } from "convex/react";
import { MockMonaco } from "features/data/components/MockMonaco.test";
import { TableSchemaContainer } from "features/data/components/TableSchema";

jest.mock("next/router", () => ({
  useRouter: jest.fn(),
}));
jest.mock("convex/react", () => ({
  useQuery: jest.fn(),
}));

const singleLineTable: [string, Shape][] = [["test", { type: "String" }]];
const multiLineTables: [string, Shape][] = [
  [
    "test",
    {
      type: "Object",
      fields: [
        {
          fieldName: "stringfield",
          optional: false,
          shape: { type: "String" },
        },
        {
          fieldName: "idfield",
          optional: true,
          shape: { type: "Id", tableName: "other" },
        },
        {
          fieldName: "bytesfield",
          optional: false,
          shape: { type: "Bytes" },
        },
      ],
    },
  ],
];

let tables: [string, Shape][] = [];

jest.mock("../lib/api", () => ({
  useTableIndexes: () => ({
    indexes: undefined,
    hadError: false,
  }),
}));

jest.mock("../../../lib/deploymentApi", () => ({
  useDeploymentUrl: () => "http://localhost",
  useDeploymentAuthHeader: () => "Bearer admin",
}));

jest.mock("../../../lib/useNents", () => ({
  useNents: () => ({
    nents: [],
    selectedNent: null,
    setSelectedNent: jest.fn(),
  }),
}));

jest.mock("../../../lib/deploymentApi", () => ({
  useTableShapes: () => ({ tables: new Map(tables) }),
}));

jest.mock("@monaco-editor/react", () => (p: any) => MockMonaco(p));

describe("TableSchema", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (useQuery as jest.Mock).mockReturnValue({});
  });

  const editorText = async () =>
    screen.getByTestId("mockMonaco").attributes.getNamedItem("value")?.value;

  const renderSchema = (query: Record<string, string> = {}) => {
    // @ts-expect-error
    useRouter.mockReturnValue({ query, replace: jest.fn() });
    return render(<TableSchemaContainer tableName="test" />);
  };

  it("adds a comment describing where other tables go", async () => {
    tables = multiLineTables;
    renderSchema();
    expect(await editorText()).toMatchInlineSnapshot(`
        "import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
          // Other tables here...

          test: defineTable({
            stringfield: v.string(),
            idfield: v.optional(v.id("other")),
            bytesfield: v.bytes(),
          }),
        });"
      `);
  });

  it("removes other tables from the schema", async () => {
    tables = [
      ["test", { type: "String" }],
      ["unrelated", { type: "String" }],
    ];
    renderSchema();
    expect(await editorText()).toMatchInlineSnapshot(`
        "import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
          // Other tables here...

          test: defineTable(v.string()),
        });"
      `);
  });

  it("it converts a single line schema to multiline", async () => {
    tables = singleLineTable;
    renderSchema();
    expect(await editorText()).toMatchInlineSnapshot(`
        "import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
          // Other tables here...

          test: defineTable(v.string()),
        });"
      `);
  });
});
