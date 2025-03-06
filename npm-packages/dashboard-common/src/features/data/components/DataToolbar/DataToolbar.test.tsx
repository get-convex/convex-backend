import { screen, render } from "@testing-library/react";
import { useRouter } from "next/router";
import userEvent from "@testing-library/user-event";
import { useMemo } from "react";
import { useToolPopup } from "@common/features/data/lib/useToolPopup";
import { useAuthorizeProdEdits } from "@common/features/data/lib/useAuthorizeProdEdits";
import {
  DataToolbar,
  DataToolbarProps,
} from "@common/features/data/components/DataToolbar/DataToolbar";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { FunctionsContext } from "@common/lib/functions/FunctionsProvider";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { PanelGroup } from "react-resizable-panels";

jest.mock("convex/react", () => ({
  useQuery: jest.fn(),
}));

jest.mock("next/router", () => ({
  useRouter: jest.fn(),
}));

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: jest.fn().mockImplementation((query) => ({
    // Always return true for media queries.
    matches: true,
    media: query,
    onchange: null,
    addListener: jest.fn(), // Deprecated
    removeListener: jest.fn(), // Deprecated
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    dispatchEvent: jest.fn(),
  })),
});

jest.mock("../../lib/api", () => ({
  useInvalidateShapes: () => jest.fn(),
  useTableIndexes: () => ({
    indexes: undefined,
    hadError: false,
  }),
}));

jest.mock("../../../../lib/useTableMetadata", () => ({
  useTableMetadata: jest.fn(),
}));
jest.mock("../../../../lib/deploymentApi", () => ({
  useDeploymentUrl: () => "http://localhost",
  useDeploymentAuthHeader: () => "Bearer admin",
}));

jest.mock("../../lib/useDefaultDocument", () => ({
  useDefaultDocument: () => jest.fn(),
}));

describe("DataToolbar", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    const mockIntersectionObserver = jest.fn();
    mockIntersectionObserver.mockReturnValue({
      observe: () => null,
      unobserve: () => null,
      disconnect: () => null,
    });
    window.IntersectionObserver = mockIntersectionObserver;
  });

  const setup = (
    hookProps: Partial<Parameters<typeof useToolPopup>[0]> = {},
    componentProps: Partial<DataToolbarProps> = {},
    query: Record<string, string> = {},
  ) => {
    // @ts-expect-error
    useRouter.mockReturnValue({ query, replace: jest.fn() });
    return render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <PanelGroup
          direction="horizontal"
          className="flex h-full grow items-stretch overflow-hidden"
        >
          <Toolbar componentProps={componentProps} hookProps={hookProps} />
        </PanelGroup>
      </DeploymentInfoContext.Provider>,
    );
  };

  function Toolbar({
    componentProps,
    hookProps,
  }: {
    hookProps: Partial<Parameters<typeof useToolPopup>[0]>;
    componentProps: Partial<DataToolbarProps>;
  }) {
    const tableName = "messages";
    const [areEditsAuthorized, onAuthorizeEdits] = useAuthorizeProdEdits({
      isProd: false,
      ...componentProps,
      ...hookProps,
    });
    const popupState = useToolPopup({
      addDocuments: jest.fn(),
      patchFields: jest.fn(),
      clearSelectedRows: jest.fn(),
      clearTable: jest.fn(),
      deleteRows: jest.fn(),
      deleteTable: jest.fn(),
      isProd: false,
      numRows: undefined,
      tableName,
      areEditsAuthorized,
      onAuthorizeEdits,
      activeSchema: null,
      ...hookProps,
    });

    return (
      <FunctionsContext.Provider value={useMemo(() => new Map(), [])}>
        {popupState.popupEl}
        <DataToolbar
          popupState={popupState}
          tableName={tableName}
          isProd={false}
          isLoadingMore={false}
          tableSchemaStatus={{
            tableName,
            isDefined: false,
            referencedByTable: undefined,
            isValidationRunning: false,
          }}
          deleteRows={jest.fn()}
          selectedRowsIds={new Set()}
          allRowsSelected={false}
          selectedDocument={undefined}
          {...hookProps}
          {...componentProps}
        />
      </FunctionsContext.Provider>
    );
  }

  it("should render content in default state", async () => {
    setup();
    expect(await screen.findByText("messages"));
    expect(screen.queryByText("documents")).toBeNull();
    expect(
      await screen.findByLabelText("Loading more documents..."),
    ).not.toBeVisible();

    const buttons = await screen.findAllByRole("button");
    expect(buttons).toHaveLength(2);

    expect(buttons[0]).toHaveTextContent("Add");
    expect(buttons[1]).toHaveAccessibleName("Open table settings");
  });

  it("should render in loading more state", async () => {
    setup({}, { isLoadingMore: true });
    expect(
      await screen.findByLabelText("Loading more documents..."),
    ).toBeVisible();
  });

  it("should open add document panel when add document button is clicked", async () => {
    const addDocuments = jest.fn();
    setup({ addDocuments });
    const addDocumentsButton = await screen.findByText("Add");
    const user = userEvent.setup();
    await user.click(addDocumentsButton);

    await screen.findByTestId("editDocumentPanel");
    expect(addDocuments).not.toHaveBeenCalled();

    const saveButton = await screen.findByRole("button", { name: "Save" });
    // We don't have a way to submit the document because we haven't mocked useDefaultDocument out.
    expect(saveButton).toBeDefined();
  });

  it("should open bulk edit panel when bulk edit button is clicked", async () => {
    const addFields = jest.fn();
    setup(
      { patchFields: addFields },
      { allRowsSelected: true, selectedRowsIds: new Set(["id1", "id2"]) },
    );
    const addFieldsButton = await screen.findByText("Edit All");
    const user = userEvent.setup();
    await user.click(addFieldsButton);

    await screen.findByTestId("editFieldsPanel");
    expect(addFields).not.toHaveBeenCalled();

    const saveButton = await screen.findByRole("button", { name: "Apply" });
    expect(saveButton).toBeDefined();
  });

  it("should open the indexes panel", async () => {
    setup();
    const menuButton = await screen.findByLabelText("Open table settings");
    const user = userEvent.setup();
    await user.click(menuButton);

    const indexes = await screen.findByText(`Schema and Indexes`);
    expect(indexes).toBeEnabled();

    await user.click(indexes);

    await screen.findByText("Schema for table");
  });

  it("should open the metrics chart", async () => {
    setup();
    const menuButton = await screen.findByLabelText("Open table settings");
    const user = userEvent.setup();
    await user.click(menuButton);

    const metrics = await screen.findByText("Metrics");
    expect(metrics).toBeEnabled();

    await user.click(metrics);

    // TODO: Write a better test for making sure the modal opens
    screen.getByTestId("modal");
  });

  it("should delete selected rows in dev", async () => {
    const deleteRows = jest.fn();
    setup(
      { deleteRows },
      {
        selectedRowsIds: new Set(["jd78w3vkw6w9q7cbv151qqxc3s6kkefa"]),
        tableSchemaStatus: undefined,
      },
    );

    const buttons = await screen.findAllByRole("button");
    expect(buttons).toHaveLength(2);

    const deleteRowsButton = buttons[1];
    expect(deleteRowsButton).toHaveTextContent("Delete");

    const user = userEvent.setup();
    await user.click(deleteRowsButton);

    expect(deleteRows).toHaveBeenCalledTimes(1);
  });

  it("should delete selected rows in prod", async () => {
    const deleteRows = jest.fn();
    setup(
      { isProd: true, deleteRows },
      {
        selectedRowsIds: new Set([
          "jd78w3vkw6w9q7cbv151qqxc3s6kkefa",
          "jd78w3vkw6w9q7cbv151qqxc3s6kkefb",
        ]),
        tableSchemaStatus: undefined,
      },
    );

    const buttons = await screen.findAllByRole("button");
    expect(buttons).toHaveLength(2);

    const deleteRowsButton = buttons[1];
    expect(deleteRowsButton).toHaveTextContent("Delete 2");

    const user = userEvent.setup();
    await user.click(deleteRowsButton);

    expect(deleteRows).toHaveBeenCalledTimes(0);

    const confirmDeleteButton = await screen.findByRole("button", {
      name: "Delete",
    });

    await user.click(confirmDeleteButton);

    expect(deleteRows).toHaveBeenCalledTimes(1);
  });

  it("should clear table in dev via selection", async () => {
    const clearTable = jest.fn();
    setup(
      { clearTable },
      {
        numRows: 2,
        allRowsSelected: true,
        selectedRowsIds: new Set(["id1", "id2"]),
      },
    );

    const user = userEvent.setup();

    const clearTableButton = await screen.findByText("Delete All");
    expect(clearTableButton).toBeEnabled();

    await user.click(clearTableButton);

    const confirmClearButton = await screen.findByRole("button", {
      name: "Confirm",
    });

    await user.click(confirmClearButton);

    // Should have cleared the table.
    expect(clearTable).toHaveBeenCalledTimes(1);
  });

  it("should clear table in dev via button", async () => {
    const clearTable = jest.fn();
    setup({ clearTable, numRows: 1 });

    const menuButton = await screen.findByLabelText("Open table settings");
    const user = userEvent.setup();
    await user.click(menuButton);

    const clearTableButton = await screen.findByText("Clear Table");
    expect(clearTableButton).toBeEnabled();

    await user.click(clearTableButton);
    const confirmClearButton = await screen.findByRole("button", {
      name: "Confirm",
    });
    await user.click(confirmClearButton);
    // Should have cleared the table.
    expect(clearTable).toHaveBeenCalledTimes(1);
  });

  it("should clear table in prod via selection", async () => {
    const clearTable = jest.fn();
    setup(
      { clearTable, isProd: true },
      {
        numRows: 2,
        allRowsSelected: true,
        selectedRowsIds: new Set(["id1", "id2"]),
      },
    );
    const user = userEvent.setup();

    const clearTableButton = await screen.findByText("Delete All");
    expect(clearTableButton).toBeEnabled();

    await user.click(clearTableButton);

    // Should not have cleared the table yes.
    expect(clearTable).toHaveBeenCalledTimes(0);

    const confirmClearButton = await screen.findByRole("button", {
      name: "Confirm",
    });
    expect(confirmClearButton).toBeDisabled();

    // Input the confirmation.
    const inputBox = await screen.findByRole("textbox");
    await user.type(inputBox, "messages");

    await user.click(confirmClearButton);

    expect(clearTable).toHaveBeenCalledTimes(1);
  });

  it("should clear table in prod via button", async () => {
    const clearTable = jest.fn();
    setup({ clearTable, isProd: true, numRows: 1 });
    const menuButton = await screen.findByLabelText("Open table settings");
    const user = userEvent.setup();
    await user.click(menuButton);

    const clearTableButton = await screen.findByText("Clear Table");
    expect(clearTableButton).toBeEnabled();

    await user.click(clearTableButton);
    // Should not have cleared the table yes.
    expect(clearTable).toHaveBeenCalledTimes(0);
    const confirmClearButton = await screen.findByRole("button", {
      name: "Confirm",
    });
    expect(confirmClearButton).toBeDisabled();
    // Input the confirmation.
    const inputBox = await screen.findByRole("textbox");
    await user.type(inputBox, "messages");
    await user.click(confirmClearButton);
    expect(clearTable).toHaveBeenCalledTimes(1);
  });

  const openMenuAndReturnDeleteTableButton = async (
    hookProps: Partial<Parameters<typeof useToolPopup>[0]> = {},
    componentProps: Partial<DataToolbarProps> = {},
  ) => {
    const deleteTable = jest.fn();
    setup({ deleteTable, ...hookProps }, componentProps);
    const menuButton = await screen.findByLabelText("Open table settings");
    const user = userEvent.setup();
    await user.click(menuButton);

    const deleteTableButton = await screen.findByText("Delete Table");
    return deleteTableButton;
  };

  it("has disabled delete table button while waiting for schemas", async () => {
    const deleteTableButton = await openMenuAndReturnDeleteTableButton(
      {},
      {
        tableSchemaStatus: {
          tableName: "messages",
          isDefined: false,
          isValidationRunning: true,
          referencedByTable: undefined,
        },
      },
    );
    expect(deleteTableButton).toBeDisabled();
  });

  it("has disabled delete table button when table in schemas", async () => {
    const deleteTableButton = await openMenuAndReturnDeleteTableButton(
      {},
      {
        tableSchemaStatus: {
          tableName: "messages",
          isDefined: true,
          isValidationRunning: false,
          referencedByTable: undefined,
        },
      },
    );
    expect(deleteTableButton).toBeDisabled();
  });

  it("has disabled delete table button when table referenced in schema", async () => {
    const deleteTableButton = await openMenuAndReturnDeleteTableButton(
      {},
      {
        tableSchemaStatus: {
          tableName: "messages",
          isDefined: false,
          isValidationRunning: false,
          referencedByTable: "users",
        },
      },
    );
    expect(deleteTableButton).toBeDisabled();
  });

  it("should delete table in dev when it's not in the schema", async () => {
    const deleteTable = jest.fn();
    const deleteTableButton = await openMenuAndReturnDeleteTableButton(
      {
        deleteTable,
      },
      {
        tableSchemaStatus: {
          tableName: "messages",
          isDefined: false,
          isValidationRunning: false,
          referencedByTable: undefined,
        },
      },
    );
    expect(deleteTableButton).toBeEnabled();

    const user = userEvent.setup();
    await user.click(deleteTableButton);

    // Should not have deleted the table.
    expect(deleteTable).toHaveBeenCalledTimes(0);

    const confirmDeleteButton = await screen.findByRole("button", {
      name: "Delete",
    });

    await user.click(confirmDeleteButton);

    // Should have deleted the table.
    expect(deleteTable).toHaveBeenCalledTimes(1);
  });

  it("should delete table in prod", async () => {
    const deleteTable = jest.fn();
    const deleteTableButton = await openMenuAndReturnDeleteTableButton(
      {
        deleteTable,
        isProd: true,
      },
      {
        tableSchemaStatus: {
          tableName: "messages",
          isDefined: false,
          isValidationRunning: false,
          referencedByTable: undefined,
        },
      },
    );
    const user = userEvent.setup();
    await user.click(deleteTableButton);

    // Should not have deleted the table.
    expect(deleteTable).toHaveBeenCalledTimes(0);

    const confirmDeleteButton = await screen.findByRole("button", {
      name: "Delete",
    });
    expect(confirmDeleteButton).toBeDisabled();

    // Input the confirmation.
    const inputBox = await screen.findByRole("textbox");
    await user.type(inputBox, "messages");

    await user.click(confirmDeleteButton);

    expect(deleteTable).toHaveBeenCalledTimes(1);
  });
});
