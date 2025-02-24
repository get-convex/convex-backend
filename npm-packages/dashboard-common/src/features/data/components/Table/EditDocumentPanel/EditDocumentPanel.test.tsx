import {
  act,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import userEvent from "@testing-library/user-event";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { MockMonaco } from "@common/features/data/components/MockMonaco.test";
import {
  EditDocumentPanel,
  useDocumentDrafts,
} from "@common/features/data/components/Table/EditDocumentPanel/EditDocumentPanel";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { PanelGroup } from "react-resizable-panels";

jest.mock("@monaco-editor/react", () => (p: any) => MockMonaco(p));
jest.mock("next/router", () => jest.requireActual("next-router-mock"));
const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

describe("EditDocumentPanel", () => {
  global.confirm = jest.fn().mockReturnValue(true);
  beforeEach(() => {
    const { result } = renderHook(() => useDocumentDrafts());
    // Reset the drafts before each test
    act(() => result.current[1]({}));

    jest.clearAllMocks();
  });

  test("renders the default document", async () => {
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={jest.fn()}
              onSave={jest.fn()}
              defaultDocument={{ ari: 1 }}
              tableName="myTable"
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    const editor = screen.getByTestId("mockMonaco");
    await waitFor(() =>
      expect(editor).toHaveDisplayValue("[  {    ari: 1,  },]"),
    );

    const user = userEvent.setup();

    const saveButton = screen.getByRole("button", { name: "Save" });
    await waitFor(() => expect(saveButton).toBeEnabled());

    await user.clear(editor);
    await user.type(editor, "invalidDoc");

    await waitFor(() => expect(editor).toHaveDisplayValue("invalidDoc"));

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Please fix the errors above to continue.",
      ),
    );
    await waitFor(() => expect(saveButton).toBeDisabled());
  });

  test("saves the document", async () => {
    const onSave = jest.fn();
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={jest.fn()}
              onSave={onSave}
              defaultDocument={{ ari: 1 }}
              tableName="myTable"
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    const editor = screen.getByTestId("mockMonaco");
    expect(editor).toHaveDisplayValue("[  {    ari: 1,  },]");

    const user = userEvent.setup();

    await user.clear(editor);
    await user.type(editor, '[[{{"ari": 2}]');
    await waitFor(() => expect(editor).toHaveDisplayValue('[{"ari": 2}]'));

    const saveButton = screen.getByRole("button", { name: "Save" });
    expect(saveButton).toBeEnabled();

    // Make sure draft state was updated
    const { result } = renderHook(() => useDocumentDrafts());
    await waitFor(() =>
      expect(result.current[0]).toEqual({ "add-null-myTable": [{ ari: 2 }] }),
    );

    await user.click(saveButton);

    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onSave).toHaveBeenCalledWith([{ ari: 2 }]);

    // Make sure the draft state got reset
    const { result: resultAfterSave } = renderHook(() => useDocumentDrafts());
    await waitFor(() => expect(resultAfterSave.current[0]).toEqual({}));
  });

  test("closes when closed in add document mode", async () => {
    const onClose = jest.fn();
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={onClose}
              onSave={jest.fn()}
              defaultDocument={{ ari: 1 }}
              tableName="myTable"
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    const closeButton = screen.getByTestId("close-panel-button");
    expect(closeButton).toBeEnabled();

    await userEvent.click(closeButton);

    expect(global.confirm).not.toHaveBeenCalled();

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  test("clears drafts when you close the panel while editing", async () => {
    const { result } = renderHook(() => useDocumentDrafts());
    expect(result.current[0]).toEqual({});
    // Simulating a draft being previously set
    await act(() => {
      result.current[1]({ "123-null-myTable": [{ _id: "123", ari: 2 }] });
    });
    const onClose = jest.fn();
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={onClose}
              onSave={jest.fn()}
              defaultDocument={{ _id: "123", ari: 1 }}
              tableName="myTable"
              editing
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    const closeButton = screen.getByTestId("close-panel-button");
    await userEvent.click(closeButton);
    expect(global.confirm).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
    // Make sure the draft state got cleared
    const { result: resultAfterClose } = renderHook(() => useDocumentDrafts());
    expect(resultAfterClose.current[0]).toEqual({});
  });

  test("keeps track of drafts when adding documents", async () => {
    const { result } = renderHook(() => useDocumentDrafts());
    expect(result.current[0]).toEqual({});

    // Simulating a draft being previously set
    act(() => {
      result.current[1]({ "add-null-myTable": [{ ari: 2 }] });
    });

    const { unmount } = render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={jest.fn()}
              onSave={jest.fn()}
              defaultDocument={{ ari: 1 }}
              tableName="myTable"
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    const editor = screen.getByTestId("mockMonaco");
    // Should load the draft into the editor
    await waitFor(() =>
      expect(editor).toHaveDisplayValue("[  {    ari: 2,  },]"),
    );

    const user = userEvent.setup();

    await user.clear(editor);
    await user.type(editor, '[[{{"ari": 3}]');
    await waitFor(() => expect(editor).toHaveDisplayValue('[{"ari": 3}]'));

    unmount();

    const { unmount: unmount2 } = render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={jest.fn()}
              onSave={jest.fn()}
              defaultDocument={{ ari: 1 }}
              tableName="notMyTable"
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    // Should display the default document for another table and not reuse the draft
    await waitFor(() =>
      expect(screen.getByTestId("mockMonaco")).toHaveDisplayValue(
        "[  {    ari: 1,  },]",
      ),
    );

    unmount2();

    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <PanelGroup
            direction="horizontal"
            className="flex h-full grow items-stretch overflow-hidden"
          >
            <EditDocumentPanel
              onClose={jest.fn()}
              onSave={jest.fn()}
              defaultDocument={{ ari: 1 }}
              tableName="myTable"
            />
          </PanelGroup>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );
    // Should display the draft for the table
    await waitFor(() =>
      expect(screen.getByTestId("mockMonaco")).toHaveDisplayValue(
        "[  {    ari: 3,  },]",
      ),
    );
  });
});
