/* eslint-disable react/button-has-type */
import React from "react";
import { act, getByRole, render } from "@testing-library/react";
import { Value } from "convex/values";
import mockRouter from "next-router-mock";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import userEvent from "@testing-library/user-event";
import { MockMonaco } from "@common/features/data/components/MockMonaco.test";
import {
  DataCell,
  DataCellProps,
} from "@common/features/data/components/Table/DataCell/DataCell";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));
jest.mock("@monaco-editor/react", () => (p: any) => MockMonaco(p));

const column = "test column";
const table = "testTable";
const value = "test value" as Value;
const docId = "1";

const defaultProps: DataCellProps = {
  value,
  document: { _id: docId },
  column: {
    Header: column,
    disableResizing: true,
  } as any,
  editDocument: jest.fn(),
  areEditsAuthorized: true,
  onAuthorizeEdits: jest.fn(),
  rowId: docId as any,
  didRowChange: false,
  width: "100px",
  inferIsDate: false,
  patchDocument: jest.fn(),
  tableName: table,
  onOpenContextMenu: jest.fn(),
  onCloseContextMenu: jest.fn(),
  canManageTable: true,
  activeSchema: null,
  isContextMenuOpen: false,
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

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.components.list, () => [
    mountedComponent,
    unmountedComponent,
  ])
  .registerQueryFake(udfs.getTableMapping.default, () => ({
    10001: "testTable",
    539: "_scheduled_jobs",
    540: "_file_storage",
  }));

describe("DataCell", () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    jest.clearAllMocks();
    mockRouter.setCurrentUrl("/");
    mockRouter.query = {
      filters: undefined,
      team: "myTeam",
      project: "myProject",
      deploymentName: "myDeployment",
    };
    global.ResizeObserver = jest.fn().mockImplementation(() => ({
      observe: jest.fn(),
      unobserve: jest.fn(),
      disconnect: jest.fn(),
    }));
    user = userEvent.setup();
  });

  const renderWithProvider = (props: Partial<DataCellProps> = {}) =>
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <DataCell {...defaultProps} {...props} />
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

  it("renders without crashing", () => {
    const { getByText } = renderWithProvider();
    expect(getByText("test value")).toBeInTheDocument();
  });

  describe("edit value", () => {
    it("opens the cell editor when button is double clicked", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      expect(button).not.toBeDisabled();
      await user.dblClick(button);
      getByTestId("cell-editor-popper");
    });

    it("opens the cell editor when pressing enter", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      await user.keyboard("{enter}");
      getByTestId("cell-editor-popper");
    });

    it("should show confirmation modal when editing a production document", async () => {
      const onAuthorizeEdits = jest.fn();
      const { getByTestId, queryByTestId } = renderWithProvider({
        onAuthorizeEdits,
        areEditsAuthorized: false,
      });
      const button = getByTestId("cell-editor-button");
      expect(button).not.toBeDisabled();
      await user.dblClick(button);
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
      const confirmModal = getByTestId("modal");
      expect(confirmModal).toBeInTheDocument();
      await user.click(getByTestId("confirm-button"));
      expect(onAuthorizeEdits).toHaveBeenCalled();
    });

    it("should show confirmation modal when editing a production document pressing enter", async () => {
      const onAuthorizeEdits = jest.fn();
      const { getByTestId, queryByTestId } = renderWithProvider({
        onAuthorizeEdits,
        areEditsAuthorized: false,
      });
      const button = getByTestId("cell-editor-button");

      await user.click(button);
      await user.keyboard("{enter}");
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();

      const confirmModal = getByTestId("modal");
      expect(confirmModal).toBeInTheDocument();
      await user.click(getByTestId("confirm-button"));
      expect(onAuthorizeEdits).toHaveBeenCalled();
    });

    it("should close the cell editor when clicking outside", async () => {
      const { getByTestId, queryByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.dblClick(button);
      getByTestId("cell-editor-popper");
      await user.click(document.body);
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
    });

    it("should close the cell editor when pressing escape", async () => {
      const { getByTestId, queryByTestId } = renderWithProvider({});

      const button = getByTestId("cell-editor-button");
      await user.dblClick(button);
      const popper = getByTestId("cell-editor-popper");
      // Normally monaco will auto focus on the editor,
      // but we need to manually focus on the popper
      // because tests are not running the entire monaco editor
      // lifecycle
      popper.focus();

      await user.keyboard("{Escape}");
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
    });

    it("is disabled when cannot manage the table", async () => {
      const { getByTestId, queryByTestId } = renderWithProvider({
        canManageTable: false,
      });
      const button = getByTestId("cell-editor-button");
      await user.dblClick(button);
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
    });

    it("is disabled for system field", async () => {
      const { getByTestId, queryByTestId } = renderWithProvider({
        column: { Header: "_id", disableResizing: true } as any,
      });
      const button = getByTestId("cell-editor-button");
      await user.dblClick(button);
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
    });

    it("is not disabled in mounted component", async () => {
      mockRouter.query = {
        filters: undefined,
        component: mountedComponent.id,
      };
      const { getByTestId } = renderWithProvider();
      const button = getByTestId("cell-editor-button");
      await user.dblClick(button);
      getByTestId("cell-editor-popper");
    });

    it("is disabled in unmounted component", async () => {
      mockRouter.query = {
        filters: undefined,
        component: unmountedComponent.id,
      };
      const { getByTestId, queryByTestId } = renderWithProvider();
      const button = getByTestId("cell-editor-button");
      await user.dblClick(button);
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
    });
  });

  describe("edit document", () => {
    it("calls the edit document prop when shift+enter is pressed", async () => {
      const { getByTestId, queryByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      await user.keyboard("{Shift>}{Enter}");
      expect(queryByTestId("cell-editor-popper")).not.toBeInTheDocument();
      expect(defaultProps.editDocument).toHaveBeenCalledTimes(1);
    });
  });

  describe("view value", () => {
    it("opens the view panel when space is pressed", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      await user.keyboard("{ }");
      getByTestId("cell-detail");
    });

    it("does not open for a reference link", async () => {
      const { getByTestId, queryByTestId } = renderWithProvider({
        value: "j57bynpqhgjdjcfm2dxpj3j7vx774s1h",
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      await user.keyboard("{ }");
      expect(queryByTestId("cell-detail")).not.toBeInTheDocument();
    });
  });

  describe("view document", () => {
    it("opens the view panel when pressing shift+space", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      await user.keyboard("{Shift>}{ }");
      getByTestId("cell-detail-document");
    });
  });

  describe("copy value", () => {
    it("copies the value to the clipboard when pressing ctrl+c", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest
        .spyOn(navigator.clipboard, "writeText")
        .mockImplementation(() => Promise.resolve());
      await user.keyboard("{Control>}c");
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("test value");
      expect(getByTestId("copied-popper")).toHaveTextContent(
        "Copied test column",
      );

      // TODO: test the tooltip disappears after a timeout
    });

    it("copies the value to the clipboard when pressing meta+c", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest
        .spyOn(navigator.clipboard, "writeText")
        .mockImplementation(() => Promise.resolve());
      await user.keyboard("{Meta>}c");
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("test value");
      expect(getByTestId("copied-popper")).toHaveTextContent(
        "Copied test column",
      );
    });

    it("copies a more complex value to the clipboard when pressing ctrl+c", async () => {
      const { getByTestId } = renderWithProvider({
        value: { key: "value", number: BigInt(3) },
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest
        .spyOn(navigator.clipboard, "writeText")
        .mockImplementation(() => Promise.resolve());
      await user.keyboard("{Control>}c");
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        '{ key: "value", number: 3n }',
      );
      expect(getByTestId("copied-popper")).toHaveTextContent(
        "Copied test column",
      );
    });
  });

  describe("copy document", () => {
    it("copies the document to the clipboard when pressing ctrl+shift+c", async () => {
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest
        .spyOn(navigator.clipboard, "writeText")
        .mockImplementation(() => Promise.resolve());
      await user.keyboard("{Control>}{Shift>}c");
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        '{ _id: "1" }',
      );
      expect(getByTestId("copied-popper")).toHaveTextContent("Copied document");
    });
  });

  describe("go to document", () => {
    it("navigates to the document when pressing ctrl+g", async () => {
      const { getByTestId } = renderWithProvider({
        value: "j57bynpqhgjdjcfm2dxpj3j7vx774s1h",
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest.spyOn(window, "open").mockImplementation(() => null);
      await user.keyboard("{Control>}g");
      expect(window.open).toHaveBeenCalledTimes(1);
      expect(window.open).toHaveBeenCalledWith(
        "http://localhost/data?table=testTable&filters=eyJjbGF1c2VzIjpbeyJpZCI6IjAiLCJmaWVsZCI6Il9pZCIsIm9wIjoiZXEiLCJ2YWx1ZSI6Imo1N2J5bnBxaGdqZGpjZm0yZHhwajNqN3Z4Nzc0czFoIn1dfQ",
        "_blank",
      );
    });

    it("navigates to the document when pressing meta+g", async () => {
      const { getByTestId } = renderWithProvider({
        value: "j57bynpqhgjdjcfm2dxpj3j7vx774s1h",
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest.spyOn(window, "open").mockImplementation(() => null);
      await user.keyboard("{Meta>}g");
      expect(window.open).toHaveBeenCalledTimes(1);
      expect(window.open).toHaveBeenCalledWith(
        "http://localhost/data?table=testTable&filters=eyJjbGF1c2VzIjpbeyJpZCI6IjAiLCJmaWVsZCI6Il9pZCIsIm9wIjoiZXEiLCJ2YWx1ZSI6Imo1N2J5bnBxaGdqZGpjZm0yZHhwajNqN3Z4Nzc0czFoIn1dfQ",
        "_blank",
      );
    });

    it("navigates to the files page when pressing meta+g on a file id", async () => {
      const { getByTestId } = renderWithProvider({
        value: "kg267e113cftx1jpeepypezsa57q9wvp",
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest.spyOn(window, "open").mockImplementation(() => null);
      await user.keyboard("{Meta>}g");
      expect(window.open).toHaveBeenCalledTimes(1);
      expect(window.open).toHaveBeenCalledWith(
        "http://localhost/files?id=kg267e113cftx1jpeepypezsa57q9wvp",
        "_blank",
      );
    });

    it("navigates to the scheduled functions page when pressing meta+g on a scheduled function id", async () => {
      const { getByTestId } = renderWithProvider({
        value: "kc2f44mqfb0kgr6dbnqwpeb2bs7q8bds",
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest.spyOn(window, "open").mockImplementation(() => null);
      await user.keyboard("{Meta>}g");
      expect(window.open).toHaveBeenCalledTimes(1);
      expect(window.open).toHaveBeenCalledWith(
        "http://localhost/schedules/functions",
        "_blank",
      );
    });

    it("does not navigate to the document when pressing ctrl+g for a non-id value", async () => {
      const { getByTestId } = renderWithProvider();
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      jest.spyOn(window, "open").mockImplementation(() => null);
      await user.keyboard("{Control>}g");
      expect(window.open).not.toHaveBeenCalled();
    });
  });

  describe("animate on change", () => {
    it("should animate on change", async () => {
      const { getByTestId, rerender } = renderWithProvider({
        didRowChange: false,
      });
      const cell = getByTestId("cell-editor-button");
      expect(cell).not.toHaveClass("animate-highlight");

      rerender(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <DataCell
              {...defaultProps}
              didRowChange={false}
              value="new value"
            />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      );

      const cellAfter = getByTestId("cell-editor-button");
      expect(cellAfter).toHaveClass("animate-highlight");

      // TODO: test the animation disappears after a timeout
    });

    it("should not animate on change if the value did not change", async () => {
      const { getByTestId, rerender } = renderWithProvider({
        didRowChange: false,
      });
      const cell = getByTestId("cell-editor-button");
      expect(cell).not.toHaveClass("animate-highlight");

      rerender(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <DataCell
              {...defaultProps}
              didRowChange={false}
              value="test value"
            />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      );

      const cellAfter = getByTestId("cell-editor-button");
      expect(cellAfter).not.toHaveClass("animate-highlight");
    });
  });

  describe("context menu", () => {
    it("should not have selected state if the context menu is open", () => {
      const { getByTestId } = renderWithProvider({ isContextMenuOpen: false });
      const button = getByTestId("cell-editor-button");
      expect(button).not.toHaveClass("ring-1");
      expect(button).not.toHaveClass("ring-border-selected");
    });
    it("should have selected state if the context menu is open", () => {
      const { getByTestId } = renderWithProvider({ isContextMenuOpen: true });
      const button = getByTestId("cell-editor-button");
      expect(button).toHaveClass("ring-1");
      expect(button).toHaveClass("ring-border-selected");
    });

    it("shows context menu on button click", async () => {
      const onOpenContextMenu = jest.fn();
      const { getByTestId } = renderWithProvider({ onOpenContextMenu });
      act(() => {
        getByTestId("cell-editor-button").focus();
      });
      const button = getByTestId("cell-context-menu-button");

      expect(onOpenContextMenu).not.toHaveBeenCalled();
      await user.click(button);
      expect(onOpenContextMenu).toHaveBeenCalledTimes(1);
      const { lastCall } = onOpenContextMenu.mock;
      expect(lastCall[0]).toEqual({
        x: expect.any(Number),
        y: expect.any(Number),
      });
      expect(lastCall[1]).toEqual(docId);
      expect(lastCall[2].column).toEqual(column);
      expect(lastCall[2].value).toEqual(value);
      expect(lastCall[2].callbacks).toBeDefined();
    });

    it("shows context menu on button click with idReferenceLink", async () => {
      const idValue = "j57bynpqhgjdjcfm2dxpj3j7vx774s1h";
      const onOpenContextMenu = jest.fn();
      const { getByTestId } = renderWithProvider({
        onOpenContextMenu,
        value: idValue,
      });
      act(() => {
        getByTestId("cell-editor-button").focus();
      });
      const button = getByTestId("cell-context-menu-button");

      expect(onOpenContextMenu).not.toHaveBeenCalled();
      await user.click(button);
      expect(onOpenContextMenu).toHaveBeenCalledTimes(1);
      const { lastCall } = onOpenContextMenu.mock;
      expect(lastCall[0]).toEqual({
        x: expect.any(Number),
        y: expect.any(Number),
      });
      expect(lastCall[1]).toEqual(docId);
      expect(lastCall[2].column).toEqual(column);
      expect(lastCall[2].value).toEqual(idValue);
      expect(lastCall[2].callbacks).toBeDefined();
    });

    it("shows context menu when right clicking", async () => {
      const onOpenContextMenu = jest.fn();
      const { getByTestId } = renderWithProvider({ onOpenContextMenu });
      const button = getByTestId("cell-editor-button");

      expect(onOpenContextMenu).not.toHaveBeenCalled();
      await user.pointer({
        target: button,
        keys: "[MouseRight]",
        coords: { x: 15, y: 10 },
      });
      expect(onOpenContextMenu).toHaveBeenCalledTimes(1);
      const { lastCall } = onOpenContextMenu.mock;
      expect(lastCall[0]).toEqual({
        x: 15,
        y: 10,
      });
    });

    it("shows the context menu when pressing ctrl+enter", async () => {
      const onOpenContextMenu = jest.fn();
      const { getByTestId } = renderWithProvider({ onOpenContextMenu });
      const button = getByTestId("cell-editor-button");

      expect(onOpenContextMenu).not.toHaveBeenCalled();
      await user.click(button);
      await user.keyboard("{Control>}{Enter}");
      expect(onOpenContextMenu).toHaveBeenCalledTimes(1);
    });

    it("shows the context menu when pressing meta+enter", async () => {
      const onOpenContextMenu = jest.fn();
      const { getByTestId } = renderWithProvider({ onOpenContextMenu });
      const button = getByTestId("cell-editor-button");

      expect(onOpenContextMenu).not.toHaveBeenCalled();
      await user.click(button);
      await user.keyboard("{Meta>}{Enter}");
      expect(onOpenContextMenu).toHaveBeenCalledTimes(1);
    });
  });

  describe("pasting values", () => {
    it("should paste a value and open the cell editor", async () => {
      jest.spyOn(window, "addEventListener");
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      expect(window.addEventListener).toHaveBeenCalledWith(
        "paste",
        expect.any(Function),
      );
      await user.paste('"pasted"');
      const editor = getByTestId("cell-editor-popper");
      expect(getByRole(editor, "textbox")).toHaveValue('"pasted"');
    });

    it("should paste a more complex value", async () => {
      jest.spyOn(window, "addEventListener");
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      expect(window.addEventListener).toHaveBeenCalledWith(
        "paste",
        expect.any(Function),
      );
      await user.paste('{ key: "123" }');
      const editor = getByTestId("cell-editor-popper");
      expect(getByRole(editor, "textbox")).toHaveValue('{ key: "123" }');
    });

    it("should paste value as string if it is not a valid vaue", async () => {
      jest.spyOn(window, "addEventListener");
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      expect(window.addEventListener).toHaveBeenCalledWith(
        "paste",
        expect.any(Function),
      );
      await user.paste("invalid");
      const editor = getByTestId("cell-editor-popper");
      expect(getByRole(editor, "textbox")).toHaveValue('"invalid"');
    });

    it("should paste undefined value", async () => {
      jest.spyOn(window, "addEventListener");
      const { getByTestId } = renderWithProvider({});
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      expect(window.addEventListener).toHaveBeenCalledWith(
        "paste",
        expect.any(Function),
      );
      await user.paste("undefined");
      const editor = getByTestId("cell-editor-popper");
      expect(getByRole(editor, "textbox")).toHaveValue("");
      expect(getByTestId("undefined-placeholder")).toHaveTextContent("unset");
    });

    it("should paste undefined as a string if the validator does not allow top level undefined", async () => {
      jest.spyOn(window, "addEventListener");
      const { getByTestId } = renderWithProvider({
        activeSchema: {
          schemaValidation: true,
          tables: [
            {
              tableName: table,
              documentType: {
                type: "object",
                value: {
                  [column]: {
                    fieldType: {
                      type: "string",
                    },
                    optional: false,
                  },
                },
              },
              indexes: [],
              searchIndexes: [],
            },
          ],
        },
      });
      const button = getByTestId("cell-editor-button");
      await user.click(button);
      expect(window.addEventListener).toHaveBeenCalledWith(
        "paste",
        expect.any(Function),
      );
      await user.paste("undefined");
      const editor = getByTestId("cell-editor-popper");
      expect(getByRole(editor, "textbox")).toHaveValue('"undefined"');
    });
  });

  describe("inferIsDate", () => {
    it("should render as date for an inferred date", async () => {
      const time = new Date("2024-12-31").getTime();
      const { getByTestId } = renderWithProvider({
        inferIsDate: true,
        value: time,
      });
      const button = getByTestId("cell-editor-button");
      expect(button).not.toHaveTextContent(time.toString());
    });

    it("should not render as date for a non-inferred value", async () => {
      const { getByTestId } = renderWithProvider({
        inferIsDate: false,
        value: new Date("2024-12-31").getTime(),
      });
      const button = getByTestId("cell-editor-button");
      expect(button).toHaveTextContent(`${new Date("2024-12-31").getTime()}`);
    });
  });

  // TODO: Move to table tests
  // These tests are disabled for now because they are slow in ci
  //   describe("arrow key navigation", () => {
  //     const renderRows = () =>
  //       render(
  //         <div>
  //           <span>
  //             <span>
  //               <ConvexProvider client={mockClient}>
  //                 <DataCell {...defaultProps} />
  //               </ConvexProvider>
  //             </span>
  //             <span>
  //               <ConvexProvider client={mockClient}>
  //                 <DataCell {...defaultProps} />
  //               </ConvexProvider>
  //             </span>
  //             <span>
  //               <ConvexProvider client={mockClient}>
  //                 <DataCell {...defaultProps} />
  //               </ConvexProvider>
  //             </span>
  //           </span>
  //           <span>
  //             <span>
  //               <ConvexProvider client={mockClient}>
  //                 <DataCell {...defaultProps} />
  //               </ConvexProvider>
  //             </span>
  //             <span>
  //               <ConvexProvider client={mockClient}>
  //                 <DataCell {...defaultProps} />
  //               </ConvexProvider>
  //             </span>
  //             <span>
  //               <ConvexProvider client={mockClient}>
  //                 <DataCell {...defaultProps} />
  //               </ConvexProvider>
  //             </span>
  //           </span>
  //         </div>,
  //       );

  //     it("should navigate to the left cell when pressing ArrowLeft", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[1]);
  //       await user.keyboard("{ArrowLeft}");
  //       expect(buttons[0]).toHaveFocus();
  //     });

  //     it("should not navigate to the left cell when pressing ArrowLeft on the first cell", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[0]);
  //       await user.keyboard("{ArrowLeft}");
  //       expect(buttons[0]).toHaveFocus();
  //     });

  //     it("should navigate to the right cell when pressing ArrowRight", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[0]);
  //       await user.keyboard("{ArrowRight}");
  //       expect(buttons[1]).toHaveFocus();
  //     });

  //     it("should not navigate to the right cell when pressing ArrowRight on the last cell", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[buttons.length - 1]);
  //       await user.keyboard("{ArrowRight}");
  //       expect(buttons[buttons.length - 1]).toHaveFocus();
  //     });

  //     it("should navigate to the cell above when pressing ArrowUp", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[3]);
  //       await user.keyboard("{ArrowUp}");
  //       expect(buttons[0]).toHaveFocus();
  //     });

  //     it("should not navigate to the cell above when pressing ArrowUp on the first row", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[0]);
  //       await user.keyboard("{ArrowUp}");
  //       expect(buttons[0]).toHaveFocus();
  //     });

  //     it("should navigate to the cell below when pressing ArrowDown", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[0]);
  //       await user.keyboard("{ArrowDown}");
  //       expect(buttons[3]).toHaveFocus();
  //     });

  //     it("should not navigate to the cell below when pressing ArrowDown on the last row", async () => {
  //       const { getAllByTestId } = renderRows();
  //       const buttons = getAllByTestId("cell-editor-button");
  //       await user.click(buttons[buttons.length - 1]);
  //       await user.keyboard("{ArrowDown}");
  //       expect(buttons[buttons.length - 1]).toHaveFocus();
  //     });
  //   });

  //   it("should render DataCell within acceptable time", () => {
  //     const start = performance.now();
  //     renderWithProvider();
  //     const end = performance.now();
  //     const renderTime = end - start;

  //     // eslint-disable-next-line no-console
  //     console.log(`DataCell render time: ${renderTime}ms`);
  //   });
});
