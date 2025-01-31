import { ConvexProvider } from "convex/react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useSet } from "react-use";
import { useResizeColumns, useTable } from "react-table";
import * as nextRouter from "next/router";
import { GenericDocument } from "convex/server";
import udfs from "udfs";
import { useMemo } from "react";
import { useDataColumns } from "features/data/components/Table/utils/useDataColumns";
import { DataRow } from "features/data/components/Table/DataRow";
import { ConnectedDeploymentContext } from "lib/deploymentContext";
import { mockConvexReactClient } from "lib/mockConvexReactClient";

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
    <ConvexProvider client={mockClient}>
      <InnerContainer initialState={initialState} />
    </ConvexProvider>
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
