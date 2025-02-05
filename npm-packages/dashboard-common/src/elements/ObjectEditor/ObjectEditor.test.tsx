import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ConvexProvider, ConvexReactClient } from "convex/react";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { useEffectOnce } from "react-use";
import udfs from "udfs";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import {
  ObjectEditor,
  ObjectEditorProps,
} from "@common/elements/ObjectEditor/ObjectEditor";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));

const tables = new Map();
tables.set("my_table", {});

const setModelMarkers = jest.fn();
jest.mock(
  "@monaco-editor/react",
  () =>
    function MockEditor({
      onChange,
      defaultValue,
      beforeMount,
      path,
    }: {
      onChange: (v: string) => void;
      defaultValue: string;
      beforeMount: (monaco: any) => void;
      path: string;
    }) {
      useEffectOnce(() => {
        beforeMount({
          MarkerSeverity: { Error: 8, Hint: 1 },
          editor: {
            getModels: () => [{ uri: { path: `/${path}` } }],
            setModelMarkers,
          },
          languages: {
            typescript: {
              javascriptDefaults: { setDiagnosticsOptions: () => {} },
            },
          },
        });
      });
      return (
        <input
          defaultValue={defaultValue}
          onChange={(e) => {
            onChange(e.target.value);
          }}
        />
      );
    },
);

const onChange = jest.fn();
const onError = jest.fn();

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0")
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => {});

describe("ObjectEditor", () => {
  beforeEach(jest.clearAllMocks);
  let editor: HTMLInputElement;

  const setup = (
    props?: Partial<ObjectEditorProps>,
    convexClient: ConvexReactClient = mockClient,
  ) => {
    render(
      <ConvexProvider client={convexClient}>
        <ObjectEditor
          defaultValue={null}
          onChange={onChange}
          onError={onError}
          path="doc"
          mode="editField"
          {...props}
        />
      </ConvexProvider>,
    );

    editor = screen.getByRole("textbox");
  };

  it("should accept valid input", async () => {
    setup();
    const user = userEvent.setup();

    await user.clear(editor);
    await user.type(editor, '"x"');
    expect(onChange).toHaveBeenCalledWith("x");
    expect(onError).toHaveBeenLastCalledWith([]);
    expect(setModelMarkers).toHaveBeenLastCalledWith(
      { uri: { path: "/doc" } },
      "",
      [],
    );
  });

  it("should error on invalid input", async () => {
    setup();
    const user = userEvent.setup();

    await user.clear(editor);
    await user.type(editor, "x");
    expect(onChange).toHaveBeenLastCalledWith(UNDEFINED_PLACEHOLDER);
    expect(onError).toHaveBeenCalledWith(["`x` is not a valid Convex value"]);
  });
});
