import { screen, render } from "@testing-library/react";
import { useRouter } from "next/router";
import { Shape } from "shapes";
import { useQuery } from "convex/react";
import { MockMonaco } from "components/MockMonaco";
import { TableSchemaContainer } from "./TableSchema";

jest.mock("next/router", () => ({
  useRouter: jest.fn(),
}));
jest.mock("convex/react", () => ({
  useQuery: jest.fn(),
}));
jest.mock("api/profile", () => {});

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

jest.mock("../../hooks/deploymentApi", () => ({
  useTableIndexes: () => ({
    indexes: undefined,
    hadError: false,
  }),
}));

jest.mock("dashboard-common", () => ({
  ...jest.requireActual("dashboard-common"),
  useDeploymentUrl: () => "https://deployment-url.com",
  useDeploymentAuthHeader: () => "auth-header",
  useLogDeploymentEvent: jest.fn(),
  useTableShapes: () => ({ tables: new Map(tables) }),
  useNents: () => ({
    nents: [],
    selectedNent: null,
    setSelectedNent: jest.fn(),
  }),
}));

jest.mock("@monaco-editor/react", () => MockMonaco);

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
