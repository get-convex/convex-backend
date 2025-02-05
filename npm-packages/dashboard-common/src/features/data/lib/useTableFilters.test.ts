import { renderHook, act } from "@testing-library/react";
import mockRouter from "next-router-mock";
import { encodeURI } from "js-base64";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import {
  useFilterMap,
  useTableFilters,
} from "@common/features/data/lib/useTableFilters";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));
describe("useTableFilters", () => {
  beforeEach(() => {
    jest.resetAllMocks();
    mockRouter.setCurrentUrl("/your-url");
    mockRouter.query = { filters: undefined };

    // Render useFilterMap and reset the global state
    const { result } = renderHook(() => useFilterMap());
    act(() => {
      result.current[1]({});
    });
  });

  it("should initialize with no filters", () => {
    const { result } = renderHook(() => useTableFilters("table1", null));

    expect(result.current.filters).toBeUndefined();
    expect(result.current.hasFilters).toBe(false);
  });

  it("should update filters", async () => {
    const { result } = renderHook(() => useTableFilters("table1", null));
    const newFilters: FilterExpression = {
      clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
    };

    await act(async () => {
      await result.current.changeFilters(newFilters);
    });

    expect(result.current.filters).toEqual(newFilters);
    expect(result.current.hasFilters).toBe(true);
  });

  it("should validate filters", async () => {
    const { result } = renderHook(() => useTableFilters("table1", null));
    const validFilters: FilterExpression = {
      clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
    };
    const invalidFilters: FilterExpression = {
      clauses: [{ op: "eq", field: undefined, id: "" }],
    };
    const noFilters: FilterExpression = {
      clauses: [],
    };

    await act(async () => {
      await result.current.changeFilters(validFilters);
    });
    expect(result.current.hasFilters).toBe(true);

    await act(async () => {
      await result.current.changeFilters(invalidFilters);
    });
    expect(result.current.hasFilters).toBe(false);

    await act(async () => {
      await result.current.changeFilters(noFilters);
    });
    expect(result.current.hasFilters).toBe(false);
  });

  it("should preserve filter state when switching between tables", async () => {
    const table1 = "table1";
    const table2 = "table2";
    const filtersTable1: FilterExpression = {
      clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
    };

    // Render the hook with table1.
    const { result, rerender } = renderHook(
      (tableName) => useTableFilters(tableName, null),
      {
        initialProps: table1,
      },
    );
    await act(async () => {
      await result.current.changeFilters(filtersTable1);
    });

    // The filters should be the same as the filters for table1.
    expect(result.current.filters).toEqual(filtersTable1);

    // Set this manually because query filters are usually unset when switching tables by useTableMetadata
    mockRouter.query.filters = undefined;
    // Switch to table2.
    rerender(table2);

    // The filters should be undefined because table2 has no filters.
    expect(result.current.filters).toBeUndefined();

    // Switch back to table1.
    rerender(table1);

    // The filters should be the same as the filters for table1.
    expect(result.current.filters).toEqual(filtersTable1);

    // Should update the query parameter with the filters for table1.
    expect(mockRouter.query.filters).toEqual(
      encodeURI(JSON.stringify(filtersTable1)),
    );
  });

  it("should use filters from the query parameter on mount", () => {
    const tableName = "table1";
    const queryFilters: FilterExpression = {
      clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
    };

    // Set up the initial state with the query filters.
    mockRouter.query.filters = encodeURI(JSON.stringify(queryFilters));

    // Render the hook with the table name.
    const { result } = renderHook(() => useTableFilters(tableName, null));

    // The filters should be the same as the query filters.
    expect(result.current.filters).toEqual(queryFilters);
  });

  it("should replace stored filters if there are query filters set when the table is changed.", () => {
    const table1 = "table1";
    const table2 = "table2";
    const filtersTable1: FilterExpression = {
      clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
    };
    const filtersTable2: FilterExpression = {
      clauses: [{ op: "eq", field: "field2", id: "", value: "" }],
    };

    mockRouter.query.filters = encodeURI(JSON.stringify(filtersTable1));

    const { result, rerender } = renderHook(
      (tableName) => useTableFilters(tableName, null),
      {
        initialProps: table1,
      },
    );

    expect(result.current.filters).toEqual(filtersTable1);

    mockRouter.query.filters = undefined;

    rerender(table2);

    expect(result.current.filters).toEqual(undefined);

    mockRouter.query.filters = encodeURI(JSON.stringify(filtersTable2));
    rerender(table1);

    expect(result.current.filters).toEqual(filtersTable2);
  });

  it("should clear out invalid query filters on mount", () => {
    const tableName = "table1";
    const queryFilters = "invalid-filters";

    // Set up the initial state with the query filters.
    mockRouter.query.filters = queryFilters;

    // Render the hook with the table name.
    const { result } = renderHook(() => useTableFilters(tableName, null));

    // The filters should be undefined because the query filters are invalid.
    expect(result.current.filters).toBeUndefined();
  });

  it("should clear out filters when the filter has empty clauses", async () => {
    const tableName = "table1";
    const newFilters: FilterExpression = {
      clauses: [],
    };

    // Render the hook with the table name.
    mockRouter.query.filters = encodeURI(JSON.stringify(newFilters));
    const { result } = renderHook(() => useTableFilters(tableName, null));

    // The filters should be undefined because the query filters are invalid.
    expect(result.current.filters).toBeUndefined();
  });

  it("should update the query parameter when filters are changed", async () => {
    const tableName = "table1";
    const newFilters: FilterExpression = {
      clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
    };

    // Render the hook with the table name.
    const { result } = renderHook(() => useTableFilters(tableName, null));

    expect(mockRouter.query.filters).toBeUndefined();
    // Change the filters.
    await act(async () => {
      await result.current.changeFilters(newFilters);
    });

    // The query parameter should be updated with the new filters.
    expect(mockRouter.query.filters).toEqual(
      encodeURI(JSON.stringify(newFilters)),
    );
  });

  // Unfortunately, next-router-mock does not support `isReady` yet.
  // TODO: Find a new way to make this test work, or wait for next-router-mock to support `isReady`.
  // it("should update filters when router becomes ready", async () => {
  //   const tableName = "table1";
  //   const queryFilters: FilterExpression = {
  //     clauses: [{ op: "eq", field: "field1", id: "", value: "" }],
  //   };

  //   // Set up the initial state with the query filters.
  //   mockRouter.query.filters = encodeURI(JSON.stringify(queryFilters));

  //   // Initially, the router is not ready.
  //   mockRouter.isReady = false;

  //   // Render the hook with the table name.
  //   const { result, rerender } = renderHook(() => useTableFilters(tableName));

  //   // The filters should be undefined because the router is not ready.
  //   expect(result.current.filters).toBeUndefined();

  //   // Now, the router becomes ready.
  //   mockRouter.isReady = true;

  //   // Rerender the hook.
  //   rerender();

  //   // The filters should be updated with the query filters.
  //   expect(result.current.filters).toEqual(queryFilters);
  // });
});
