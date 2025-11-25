import { ConvexProvider } from "convex/react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useSet } from "react-use";
import { useResizeColumns, useTable } from "react-table";
import * as nextRouter from "next/router";
import { GenericDocument } from "convex/server";
import udfs from "@common/udfs";
import { useMemo } from "react";
import { useDataColumns } from "@common/features/data/components/Table/utils/useDataColumns";
import { DataRow } from "@common/features/data/components/Table/DataRow";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

const mockRouter = jest
  .fn()
  .mockImplementation(() => ({ route: "/", query: {} }));
(nextRouter as any).useRouter = mockRouter;

// @ts-expect-error
const mockClient: ConvexReactClient = mockConvexReactClient()
  .registerQueryFake(udfs.getTableMapping.default, () => ({}))
  .registerQueryFake(udfs.components.list, () => []);

// @ts-expect-error
const deployment: ConnectedDeployment = {};

function TestContainer({ initialState }: { initialState: boolean[] }) {
  return (
    <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
      <ConvexProvider client={mockClient}>
        <InnerContainer initialState={initialState} />
      </ConvexProvider>
    </DeploymentInfoContext.Provider>
  );
}

function InnerContainer({ initialState }: { initialState: boolean[] }) {
  const [_selectedRows, selectedRowsActions] = useSet<string>(
    new Set(
      initialState.flatMap((enabled, index) =>
        enabled ? [index.toString()] : [],
      ),
    ),
  );

  const data: GenericDocument[] = initialState.map((_, index) => ({
    _id: index.toString(),
  }));
  const columns = useDataColumns({
    tableName: "test",
    localStorageKey: "_disabled_",
    fields: ["_id"],
    data,
  });
  const { rows, prepareRow } = useTable({ columns, data }, useResizeColumns);

  const connectedDeployment = useMemo(
    () => ({ deployment, isDisconnected: false }),
    [],
  );

  return (
    <ConnectedDeploymentContext.Provider value={connectedDeployment}>
      {initialState.map((_, index) => (
        <DataRow
          key={index}
          index={index}
          style={{}}
          data={{
            areEditsAuthorized: true,
            isRowSelected: selectedRowsActions.has,
            isSelectionAllNonExhaustive: false,
            resizingColumn: undefined,
            onAuthorizeEdits: () => {},
            patchDocument: async () => undefined,
            prepareRow,
            rows,
            tableName: "foo",
            toggleIsRowSelected: selectedRowsActions.toggle,
            onOpenContextMenu: () => {},
            onCloseContextMenu: () => {},
            contextMenuColumn: null,
            contextMenuRow: null,
            canManageTable: true,
            activeSchema: null,
            onEditDocument: () => {
              /* noop */
            },
          }}
        />
      ))}
    </ConnectedDeploymentContext.Provider>
  );
}

const createRows = (initialState: boolean[]) => {
  render(<TestContainer initialState={initialState} />);
  expectRows(initialState);
};

function expectRows(expectedState: boolean[]) {
  const checkboxes = screen.queryAllByRole("checkbox");
  expect(checkboxes.length).toStrictEqual(expectedState.length);

  for (let i = 0; i < expectedState.length; i++) {
    if (expectedState[i]) {
      expect(checkboxes[i]).toBeChecked();
    } else {
      expect(checkboxes[i]).not.toBeChecked();
    }
  }
}

async function shiftToggleRow(index: number) {
  const user = userEvent.setup();
  await user.keyboard("[ShiftLeft>]");
  await user.click(screen.queryAllByRole("checkbox")[index]);
}

describe("DataRow", () => {
  const X = true;
  const _ = false;

  describe("selection", () => {
    it("should select multiple rows at once", async () => {
      //          0  1  2  3  4  5  6  7  8  9
      createRows([_, _, _, X, _, _, _, _, _, _]);
      await shiftToggleRow(7);
      expectRows([_, _, _, X, X, X, X, X, _, _]);
    });

    it("should unselect multiple rows", async () => {
      //          0  1  2  3  4  5  6  7  8  9
      createRows([_, _, _, X, X, X, X, X, _, _]);
      await shiftToggleRow(4);
      expectRows([_, _, _, X, _, _, _, _, _, _]);
    });

    it("should select from the start when nothing is selected", async () => {
      //          0  1  2  3  4  5  6  7  8  9
      createRows([_, _, _, _, _, _, _, _, _, _]);
      await shiftToggleRow(4);
      expectRows([X, X, X, X, X, _, _, _, _, _]);
    });

    it("should select from the group above when it exists", async () => {
      //          0  1  2  3  4  5  6  7  8  9
      createRows([X, X, _, _, _, _, X, X, X, _]);
      await shiftToggleRow(4);
      expectRows([X, X, X, X, X, _, X, X, X, _]);
    });

    it("should select from the group below when no group exists above", async () => {
      //          0  1  2  3  4  5  6  7  8  9
      createRows([_, _, _, _, _, _, X, X, X, _]);
      await shiftToggleRow(4);
      expectRows([_, _, _, _, X, X, X, X, X, _]);
    });

    it("should batch deselect correctly when multiple groups exist", async () => {
      //          0  1  2  3  4  5  6  7  8  9
      createRows([X, X, _, X, X, X, _, X, X, _]);
      await shiftToggleRow(4);
      expectRows([X, X, _, X, _, _, _, X, X, _]);
    });
  });

  describe("hidden _id column", () => {
    // Helper component to wrap hooks properly
    function TestRowWithHiddenId({
      data,
      fields,
      patchDocument,
      onOpenContextMenu,
      isRowSelected,
      toggleIsRowSelected,
    }: {
      data: GenericDocument[];
      fields: string[];
      patchDocument: any;
      onOpenContextMenu: any;
      isRowSelected: (id: string) => boolean;
      toggleIsRowSelected: (id: string) => void;
    }) {
      // Only include specified fields in columns, potentially hiding _id
      const columns = useDataColumns({
        tableName: "test",
        localStorageKey: "_disabled_",
        fields,
        data,
      });

      const { rows, prepareRow } = useTable(
        { columns, data },
        useResizeColumns,
      );

      const connectedDeployment = useMemo(
        () => ({ deployment, isDisconnected: false }),
        [],
      );

      return (
        <ConnectedDeploymentContext.Provider value={connectedDeployment}>
          <DataRow
            index={0}
            style={{}}
            data={{
              areEditsAuthorized: true,
              isRowSelected,
              isSelectionAllNonExhaustive: false,
              resizingColumn: undefined,
              onAuthorizeEdits: () => {},
              patchDocument,
              prepareRow,
              rows,
              tableName: "test",
              toggleIsRowSelected,
              onOpenContextMenu,
              onCloseContextMenu: () => {},
              contextMenuColumn: null,
              contextMenuRow: null,
              canManageTable: true,
              activeSchema: null,
              onEditDocument: () => {},
            }}
          />
        </ConnectedDeploymentContext.Provider>
      );
    }

    it("should access _id from row.original when _id column is hidden", () => {
      const patchDocument = jest.fn();
      const onOpenContextMenu = jest.fn();

      const data: GenericDocument[] = [
        { _id: "test-id-1", name: "John", age: 30 },
        { _id: "test-id-2", name: "Jane", age: 25 },
      ];

      const { container } = render(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <TestRowWithHiddenId
              data={data}
              fields={["name", "age"]}
              patchDocument={patchDocument}
              onOpenContextMenu={onOpenContextMenu}
              isRowSelected={() => false}
              toggleIsRowSelected={() => {}}
            />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      );

      // Verify the row renders without errors
      expect(container.querySelector(".DataRow")).toBeInTheDocument();

      // Verify that we can see the name and age values
      expect(container).toHaveTextContent("John");
      expect(container).toHaveTextContent("30");
    });

    it("should correctly identify rows by _id when _id column is hidden", () => {
      const isRowSelected = jest.fn(() => false);
      const toggleIsRowSelected = jest.fn();

      const data: GenericDocument[] = [
        { _id: "test-id-1", name: "John", age: 30 },
        { _id: "test-id-2", name: "Jane", age: 25 },
      ];

      render(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <TestRowWithHiddenId
              data={data}
              fields={["name", "age"]}
              patchDocument={async () => undefined}
              onOpenContextMenu={() => {}}
              isRowSelected={isRowSelected}
              toggleIsRowSelected={toggleIsRowSelected}
            />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      );

      // Verify isRowSelected was called with the correct _id
      expect(isRowSelected).toHaveBeenCalledWith("test-id-1");
    });

    it("should open context menu with correct _id when _id column is hidden", async () => {
      const onOpenContextMenu = jest.fn();

      const data: GenericDocument[] = [
        { _id: "test-id-1", name: "John", age: 30 },
      ];

      const { container } = render(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <TestRowWithHiddenId
              data={data}
              fields={["name", "age"]}
              patchDocument={async () => undefined}
              onOpenContextMenu={onOpenContextMenu}
              isRowSelected={() => false}
              toggleIsRowSelected={() => {}}
            />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      );

      // Find and right-click the checkbox
      const checkbox = container.querySelector('input[type="checkbox"]');
      expect(checkbox).toBeInTheDocument();

      const user = userEvent.setup();
      await user.pointer({
        target: checkbox!.parentElement!,
        keys: "[MouseRight]",
      });

      // Verify onOpenContextMenu was called with correct _id
      expect(onOpenContextMenu).toHaveBeenCalledWith(
        expect.any(Object),
        "test-id-1",
        null,
      );
    });

    it("should handle recently created row highlighting when _id column is hidden", () => {
      const data: GenericDocument[] = [
        {
          _id: "test-id-1",
          _creationTime: Date.now() - 500, // Created 500ms ago
          name: "John",
          age: 30,
        },
      ];

      const { container } = render(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <TestRowWithHiddenId
              data={data}
              fields={["name", "age"]}
              patchDocument={async () => undefined}
              onOpenContextMenu={() => {}}
              isRowSelected={() => false}
              toggleIsRowSelected={() => {}}
            />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      );

      // Verify the row renders with the highlight animation
      const row = container.querySelector(".DataRow");
      expect(row).toHaveClass("animate-highlight");
    });
  });
});
