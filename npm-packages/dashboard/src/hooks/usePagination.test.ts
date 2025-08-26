import { renderHook, act } from "@testing-library/react";
import { usePagination } from "./usePagination";

describe("usePagination", () => {
  it("paginates items and updates visibleItems on page change", () => {
    const items = Array.from({ length: 16 }, (_, i) => `item${i + 1}`);
    const itemsPerPage = 5;

    const { result } = renderHook(() => usePagination({ items, itemsPerPage }));

    // Page 1: items 0-4
    expect(result.current.visibleItems).toEqual([
      "item1",
      "item2",
      "item3",
      "item4",
      "item5",
    ]);
    expect(result.current.totalPages).toBe(4);
    expect(result.current.currentPage).toBe(1);

    // Go to last page
    act(() => {
      result.current.setCurrentPage(4);
    });

    // Page 4: items 15-16
    expect(result.current.visibleItems).toEqual(["item16"]);
    expect(result.current.currentPage).toBe(4);
  });

  it("decreases currentPage if items are removed and currentPage is out of range", () => {
    let items = Array.from({ length: 10 }, (_, i) => `item${i + 1}`);
    const itemsPerPage = 3;

    const { result, rerender } = renderHook(
      // eslint-disable-next-line @typescript-eslint/no-shadow
      ({ items }) => usePagination({ items, itemsPerPage }),
      { initialProps: { items } },
    );

    // Go to last page (should be page 4)
    act(() => {
      result.current.setCurrentPage(4);
    });
    expect(result.current.currentPage).toBe(4);
    expect(result.current.visibleItems).toEqual(["item10"]);

    // Remove items so there are only 7 left (now only 3 pages)
    items = items.slice(0, 7);
    rerender({ items });

    // currentPage should decrease to 3
    expect(result.current.currentPage).toBe(3);
    expect(result.current.visibleItems).toEqual(["item7"]);
    expect(result.current.totalPages).toBe(3);

    // Remove all items
    items = [];
    rerender({ items });

    // currentPage should be 1
    expect(result.current.currentPage).toBe(1);
    expect(result.current.visibleItems).toEqual([]);
    expect(result.current.totalPages).toBe(0);
  });
});
