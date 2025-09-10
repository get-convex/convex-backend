import { render } from "@testing-library/react";
import { GenericDocument } from "convex/server";
import mockRouter from "next-router-mock";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import userEvent from "@testing-library/user-event";
import {
  TableContextMenu,
  TableContextMenuProps,
} from "@common/features/data/components/Table/TableContextMenu";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));

const mockData: GenericDocument[] = [
  { _id: "1", name: "Document 1" },
  { _id: "2", name: "Document 2" },
];

const defaultProps: TableContextMenuProps = {
  data: mockData,
  state: {
    target: { x: 0, y: 0 },
    selectedCell: {
      rowId: "1",
      column: "name",
      value: "Document 1",
      callbacks: {
        copy: jest.fn(),
        copyDoc: jest.fn(),
        goToRef: jest.fn(),
        edit: jest.fn(),
        editDoc: jest.fn(),
        view: jest.fn(),
        viewDoc: jest.fn(),
        docRefLink: undefined,
      },
    },
  },
  close: jest.fn(),
  deleteRows: jest.fn(),
  isProd: false,
  setPopup: jest.fn(),
  onAddDraftFilter: jest.fn(),
  defaultDocument: { _id: "1", name: "" },
  resetColumns: jest.fn(),
  canManageTable: true,
};

const mountedComponent = {
  path: "mounted",
  id: "mounted" as any,
  name: null,
  args: {},
  state: "active",
} as (typeof udfs.components.list._returnType)[0];
const unmountedComponent = {
  path: "unmounted",
  id: "unmounted" as any,
  name: null,
  args: {},
  state: "unmounted",
} as (typeof udfs.components.list._returnType)[0];

const mockClient = mockConvexReactClient().registerQueryFake(
  udfs.components.list,
  () => [mountedComponent, unmountedComponent],
);

describe("TableContextMenu", () => {
  let user: ReturnType<typeof userEvent.setup>;

  const renderWithProvider = (props: Partial<TableContextMenuProps> = {}) =>
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <TableContextMenu {...defaultProps} {...props} />
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

  beforeEach(() => {
    mockRouter.setCurrentUrl("/");
    jest.clearAllMocks();
    user = userEvent.setup();
  });

  it("should have filter by column", async () => {
    const { getByText } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: {
          rowId: null,
          column: "name",
          value: undefined,
        },
      },
    });

    await user.click(getByText("Filter by"));

    expect(defaultProps.onAddDraftFilter).toHaveBeenCalledTimes(1);
    expect(defaultProps.close).toHaveBeenCalledTimes(1);
    expect(defaultProps.onAddDraftFilter).toHaveBeenCalledWith({
      field: "name",
      op: "eq",
      value: defaultProps.defaultDocument.name,
      id: expect.anything(),
      enabled: true,
    });
  });

  it("should have filter by when there is data", async () => {
    const { getByText } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: {
          rowId: "1",
          column: "name",
          value: "Document 1",
          callbacks: {
            copy: jest.fn(),
            copyDoc: jest.fn(),
            goToRef: jest.fn(),
            edit: jest.fn(),
            editDoc: jest.fn(),
            view: jest.fn(),
            viewDoc: jest.fn(),
            docRefLink: undefined,
          },
        },
      },
    });

    await user.click(getByText("Filter by"));

    expect(defaultProps.onAddDraftFilter).toHaveBeenCalledTimes(1);
    expect(defaultProps.close).toHaveBeenCalledTimes(1);
    expect(defaultProps.onAddDraftFilter).toHaveBeenCalledWith({
      field: "name",
      op: "eq",
      value: defaultProps.defaultDocument.name,
      id: expect.anything(),
    });
  });

  // TODO: Tests for "filter by" submenu

  it("should have reset columns button", async () => {
    const { getByText } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: { rowId: null, column: "name", value: undefined },
      },
    });

    expect(defaultProps.resetColumns).toHaveBeenCalledTimes(0);
    expect(defaultProps.close).toHaveBeenCalledTimes(0);

    const action = getByText("Reset column positions and widths");
    await user.click(action);

    expect(defaultProps.resetColumns).toHaveBeenCalledTimes(1);
    expect(defaultProps.close).toHaveBeenCalledTimes(1);
  });

  it("should link to the document reference", async () => {
    const { getByTestId } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: {
          rowId: "1",
          column: "name",
          value: "Document 1",
          callbacks: {
            copy: jest.fn(),
            copyDoc: jest.fn(),
            goToRef: jest.fn(),
            edit: jest.fn(),
            editDoc: jest.fn(),
            view: jest.fn(),
            viewDoc: jest.fn(),
            docRefLink: { pathname: "/document/1" },
          },
        },
      },
    });

    const link = getByTestId("table-context-menu").children[0];
    expect(link).toHaveTextContent("Go to Reference");
    expect(link).toHaveAttribute("href", "/document/1");
    expect(link).toHaveAttribute("target", "_blank");
  });

  it("should link to the scheduled functions page", async () => {
    const { getByTestId } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: {
          rowId: "1",
          column: "name",
          value: "Document 1",
          callbacks: {
            copy: jest.fn(),
            copyDoc: jest.fn(),
            goToRef: jest.fn(),
            edit: jest.fn(),
            editDoc: jest.fn(),
            view: jest.fn(),
            viewDoc: jest.fn(),
            docRefLink: { pathname: "/schedules/functions" },
          },
        },
      },
    });

    const link = getByTestId("table-context-menu").children[0];
    expect(link).toHaveTextContent("Go to Scheduled Functions");
    expect(link).toHaveAttribute("href", "/schedules/functions");
    expect(link).toHaveAttribute("target", "_blank");
  });

  it("should link to the a specific file on the Files page", async () => {
    const { getByTestId } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: {
          rowId: "1",
          column: "name",
          value: "Document 1",
          callbacks: {
            copy: jest.fn(),
            copyDoc: jest.fn(),
            goToRef: jest.fn(),
            edit: jest.fn(),
            editDoc: jest.fn(),
            view: jest.fn(),
            viewDoc: jest.fn(),
            docRefLink: {
              pathname: "/files",
              query: { id: "kg267e113cftx1jpeepypezsa57q9wvp" },
            },
          },
        },
      },
    });

    const link = getByTestId("table-context-menu").children[0];
    expect(link).toHaveTextContent("Go to File");
    expect(link).toHaveAttribute(
      "href",
      "/files?id=kg267e113cftx1jpeepypezsa57q9wvp",
    );
    expect(link).toHaveAttribute("target", "_blank");
  });

  it.each`
    callbackName | buttonIndex
    ${"view"}    | ${0}
    ${"copy"}    | ${1}
    ${"edit"}    | ${2}
    ${"viewDoc"} | ${5}
    ${"copyDoc"} | ${6}
    ${"editDoc"} | ${7}
  `(
    "should call the $callbackName callback",
    async ({ callbackName, buttonIndex }) => {
      const { getByTestId } = renderWithProvider({});

      expect(
        // @ts-ignore
        defaultProps.state?.selectedCell?.callbacks?.[callbackName],
      ).toHaveBeenCalledTimes(0);

      const button = getByTestId("table-context-menu").children[buttonIndex];
      await user.click(button);

      expect(
        // @ts-ignore
        defaultProps.state?.selectedCell?.callbacks?.[callbackName],
      ).toHaveBeenCalledTimes(1);
    },
  );

  it.each`
    hotkey                   | callbackName
    ${"{Control>}c"}         | ${"copy"}
    ${"{Meta>}c"}            | ${"copy"}
    ${" "}                   | ${"view"}
    ${"{enter}"}             | ${"edit"}
    ${"{Shift>}{ }"}         | ${"viewDoc"}
    ${"{Control>}{Shift>}c"} | ${"copyDoc"}
    ${"{Meta>}{Shift>}c"}    | ${"copyDoc"}
    ${"{Shift>}{enter}"}     | ${"editDoc"}
    ${"{Control>}g"}         | ${"goToRef"}
    ${"{Meta>}g"}            | ${"goToRef"}
  `(
    "should call the $callbackName callback when pressing $hotkey",
    async ({ hotkey, callbackName }) => {
      renderWithProvider({});

      expect(
        // @ts-ignore
        defaultProps.state?.selectedCell?.callbacks?.[callbackName],
      ).toHaveBeenCalledTimes(0);

      await user.keyboard(hotkey);

      expect(
        // @ts-ignore
        defaultProps.state?.selectedCell?.callbacks?.[callbackName],
      ).toHaveBeenCalledTimes(1);
    },
  );

  it("should call the delete document callback", async () => {
    const { getByTestId } = renderWithProvider({});

    expect(defaultProps.deleteRows).toHaveBeenCalledTimes(0);

    const button = getByTestId("table-context-menu").children[8];
    await user.click(button);

    expect(defaultProps.deleteRows).toHaveBeenCalledTimes(1);
  });

  it("should show confirmation for the delete document button when in prod", async () => {
    const { getByTestId } = renderWithProvider({ isProd: true });

    const button = getByTestId("table-context-menu").children[8];
    await user.click(button);
    expect(defaultProps.deleteRows).toHaveBeenCalledTimes(0);
    expect(defaultProps.setPopup).toHaveBeenCalledTimes(1);
    expect(defaultProps.setPopup).toHaveBeenCalledWith({
      rowIds: new Set(["1"]),
      type: "deleteRows",
    });
  });
  it("should disable the edit button when the user cannot manage the table", async () => {
    const { getByTestId } = renderWithProvider({ canManageTable: false });

    const button = getByTestId("table-context-menu").children[2];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(
      defaultProps.state?.selectedCell?.callbacks?.edit,
    ).toHaveBeenCalledTimes(0);
  });

  it("should disable the edit button when the column is _id", async () => {
    const { getByTestId } = renderWithProvider({
      state: {
        target: { x: 0, y: 0 },
        selectedCell: {
          rowId: "1",
          column: "_id",
          value: "1",
          callbacks: {
            copy: jest.fn(),
            copyDoc: jest.fn(),
            goToRef: jest.fn(),
            edit: jest.fn(),
            editDoc: jest.fn(),
            view: jest.fn(),
            viewDoc: jest.fn(),
            docRefLink: undefined,
          },
        },
      },
    });

    const button = getByTestId("table-context-menu").children[2];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(
      defaultProps.state?.selectedCell?.callbacks?.edit,
    ).toHaveBeenCalledTimes(0);
  });

  it("should disable the edit button when not in a mounted component", async () => {
    mockRouter.query = {
      filters: undefined,
      component: unmountedComponent.id,
    };
    const { getByTestId } = renderWithProvider({});

    const button = getByTestId("table-context-menu").children[2];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(
      defaultProps.state?.selectedCell?.callbacks?.edit,
    ).toHaveBeenCalledTimes(0);
  });

  it("should disable the delete document button when the user cannot manage the table", async () => {
    const { getByTestId } = renderWithProvider({ canManageTable: false });

    const button = getByTestId("table-context-menu").children[8];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(defaultProps.deleteRows).toHaveBeenCalledTimes(0);
  });

  it("should disable the delete document button when not in a mounted component", async () => {
    mockRouter.query = {
      filters: undefined,
      component: unmountedComponent.id,
    };
    const { getByTestId } = renderWithProvider({});

    const button = getByTestId("table-context-menu").children[8];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(defaultProps.deleteRows).toHaveBeenCalledTimes(0);
  });

  it("should disable the edit document button when the user cannot manage the table", async () => {
    const { getByTestId } = renderWithProvider({ canManageTable: false });

    const button = getByTestId("table-context-menu").children[2];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(
      defaultProps.state?.selectedCell?.callbacks?.edit,
    ).toHaveBeenCalledTimes(0);
  });

  it("should disable the edit document button when not in a mounted component", async () => {
    mockRouter.query = {
      filters: undefined,
      component: unmountedComponent.id,
    };
    const { getByTestId } = renderWithProvider({});

    const button = getByTestId("table-context-menu").children[2];
    expect(button).toBeDisabled();

    await user.click(button);
    expect(
      defaultProps.state?.selectedCell?.callbacks?.edit,
    ).toHaveBeenCalledTimes(0);
  });
});
