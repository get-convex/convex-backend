import { screen, render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { MockMonaco } from "@common/features/data/components/MockMonaco.test";
import {
  FilterEditor,
  FilterEditorProps,
} from "@common/features/data/components/FilterEditor/FilterEditor";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0")
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

jest.mock("@monaco-editor/react", () => (p: any) => MockMonaco(p));
jest.mock("next/router", () => jest.requireActual("next-router-mock"));

const onChange = jest.fn();

describe("FilterEditor", () => {
  beforeEach(jest.clearAllMocks);

  const setup = (props?: Partial<FilterEditorProps>) =>
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <FilterEditor
            fields={[
              "_id",
              "_creationTime",
              "myColumn",
              "anotherColumn",
              "myColumn2",
            ]}
            defaultDocument={{
              myColumn: 0,
              anotherColumn: "stringy",
              myColumn2: 0,
            }}
            onChange={onChange}
            onDelete={jest.fn()}
            onError={jest.fn()}
            onApplyFilters={jest.fn()}
            {...props}
          />
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

  it("should save valid filter", async () => {
    setup();

    const user = userEvent.setup();

    let valueInput = await screen.findByTestId("mockMonaco");
    expect(valueInput).toHaveDisplayValue("null");
    const fieldSelector = await screen.findByTestId(
      "combobox-button-Select filter field",
    );
    const operatorSelector = await screen.findByTestId(
      "combobox-button-Select filter operator",
    );

    expect(onChange).not.toHaveBeenCalled();

    await user.click(fieldSelector);
    const columnOption = await screen.findByText("myColumn");
    await user.click(columnOption);
    expect(onChange).toHaveBeenLastCalledWith({
      enabled: true,
      field: "myColumn",
      op: "eq",
      value: 0,
    });
    expect(fieldSelector).toHaveTextContent("myColumn");

    expect(operatorSelector).toHaveTextContent("equals");
    await user.click(operatorSelector);
    const operatorOption = await screen.findByText("not equal");
    await user.click(operatorOption);
    expect(operatorSelector).toHaveTextContent("not equal");
    expect(onChange).toHaveBeenLastCalledWith({
      enabled: true,
      field: "myColumn",
      op: "neq",
      value: 0,
    });

    // The old input should have been removed and replaced with a new one.
    expect(valueInput).not.toBeInTheDocument();
    valueInput = await screen.findByTestId("mockMonaco");
    expect(valueInput).toBeInTheDocument();
    expect(valueInput).toHaveDisplayValue("0");
    await user.clear(valueInput);
    expect(valueInput).toHaveDisplayValue("");
    await user.type(valueInput, "123");
    expect(valueInput).toHaveDisplayValue("123");

    expect(onChange).toHaveBeenLastCalledWith({
      enabled: true,
      field: "myColumn",
      op: "neq",
      value: 123,
    });
  });

  it("should change filter value when changing from builtin filter to type filter ", async () => {
    setup({
      defaultValue: { field: "myColumn", op: "eq", value: "123" },
    });
    const user = userEvent.setup();

    const valueInput = await screen.findByTestId("mockMonaco");
    expect(valueInput).toHaveDisplayValue('"123"');
    const fieldSelector = await screen.findByTestId(
      "combobox-button-Select filter field",
    );
    const operatorSelector = await screen.findByTestId(
      "combobox-button-Select filter operator",
    );
    expect(fieldSelector).toHaveTextContent("myColumn");
    expect(operatorSelector).toHaveTextContent("equals");

    await user.click(operatorSelector);

    const operatorOption = await screen.findByText("is type");
    await user.click(operatorOption);

    expect(valueInput).not.toBeInTheDocument();

    // Should have changed the type value
    expect(onChange).toHaveBeenLastCalledWith({
      field: "myColumn",
      op: "type",
      value: "number",
    });
    const typeSelector = await screen.findByTestId(
      "combobox-button-Select type value",
    );
    // Should display the new type
    expect(typeSelector).toHaveTextContent("number");
  });

  it("should change filter value when changing from type filter to default filter ", async () => {
    setup({
      defaultValue: { field: "myColumn", op: "type", value: "string" },
    });
    const user = userEvent.setup();

    const typeSelector = await screen.findByTestId(
      "combobox-button-Select type value",
    );
    expect(typeSelector).toHaveTextContent("string");
    const fieldSelector = await screen.findByTestId(
      "combobox-button-Select filter field",
    );
    const operatorSelector = await screen.findByTestId(
      "combobox-button-Select filter operator",
    );
    expect(fieldSelector).toHaveTextContent("myColumn");
    expect(operatorSelector).toHaveTextContent("is type");

    await user.click(operatorSelector);

    const operatorOption = await screen.findByText("equals");
    await user.click(operatorOption);

    // Should have changed the value
    expect(onChange).toHaveBeenLastCalledWith({
      field: "myColumn",
      op: "eq",
      value: 0,
    });
    // Should display the new value
    const valueInput = await screen.findByTestId("mockMonaco");
    expect(valueInput).toHaveDisplayValue("0");
  });

  it("should change filter value when changing from type filter to default filter ", async () => {
    setup({
      defaultValue: { field: "myColumn", op: "type", value: "string" },
    });
    const user = userEvent.setup();

    const typeSelector = await screen.findByTestId(
      "combobox-button-Select type value",
    );
    expect(typeSelector).toHaveTextContent("string");
    const fieldSelector = await screen.findByTestId(
      "combobox-button-Select filter field",
    );
    const operatorSelector = await screen.findByTestId(
      "combobox-button-Select filter operator",
    );
    expect(fieldSelector).toHaveTextContent("myColumn");
    expect(operatorSelector).toHaveTextContent("is type");

    await user.click(operatorSelector);

    const operatorOption = await screen.findByText("is not type");
    await user.click(operatorOption);

    // Should have changed the type value
    expect(onChange).toHaveBeenLastCalledWith({
      field: "myColumn",
      op: "notype",
      value: "string",
    });
    // Should display the old type
    expect(typeSelector).toHaveTextContent("string");
  });

  it("should change filter value when switching to a field of a different type", async () => {
    setup({
      defaultValue: { field: "myColumn", op: "type", value: "string" },
    });
    const user = userEvent.setup();

    const typeSelector = await screen.findByTestId(
      "combobox-button-Select type value",
    );
    expect(typeSelector).toHaveTextContent("string");

    await user.click(typeSelector);

    const typeOption = await screen.findByText("array");
    await user.click(typeOption);

    // Should have changed the type value
    expect(onChange).toHaveBeenLastCalledWith({
      field: "myColumn",
      op: "type",
      value: "array",
    });
    // Should display the new type
    expect(typeSelector).toHaveTextContent("array");
  });

  it("should not change filter value when switching to a field that has the same type as the value", async () => {
    setup({
      defaultValue: { field: "myColumn", op: "eq", value: 100 },
    });
    const user = userEvent.setup();

    const fieldSelector = await screen.findByTestId(
      "combobox-button-Select filter field",
    );
    expect(fieldSelector).toHaveTextContent("myColumn");

    await user.click(fieldSelector);

    const fieldOption = await screen.findByText("myColumn2");
    await user.click(fieldOption);
    expect(fieldSelector).toHaveTextContent("myColumn2");

    // Should have changed the type value
    expect(onChange).toHaveBeenLastCalledWith({
      field: "myColumn2",
      op: "eq",
      value: 100,
    });
  });

  it("should change type value when changing from builtin filter to type filter ", async () => {
    setup({
      defaultValue: { field: "myColumn", op: "type", value: "number" },
    });
    const user = userEvent.setup();

    const fieldSelector = await screen.findByTestId(
      "combobox-button-Select filter field",
    );
    expect(fieldSelector).toHaveTextContent("myColumn");

    await user.click(fieldSelector);

    const fieldOption = await screen.findByText("anotherColumn");
    await user.click(fieldOption);

    // Should have changed the type value
    expect(onChange).toHaveBeenLastCalledWith({
      field: "anotherColumn",
      op: "type",
      value: "string",
    });
    const typeSelector = await screen.findByTestId(
      "combobox-button-Select type value",
    );
    // Should display the new type
    expect(typeSelector).toHaveTextContent("string");
  });
});
