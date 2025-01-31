import { fireEvent, render } from "@testing-library/react";
import { useRef } from "react";
import { useMaintainScrollPositionOnChange } from "features/data/components/Table/utils/useMaintainScrollPositionOnChange";

const ROW_HEIGHT = 24;

describe("useMaintainScrollPositionOnChange", () => {
  it("doesn’t change the scroll position when the user hasn’t scrolled down the list", () => {
    const onRowChangeAbove = jest.fn();

    const { rerender, getByTestId } = render(
      <TestContainer
        rows={[
          "Row 1",
          "Row 2",
          "Row 3",
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );
    const list = getByTestId("scroll");

    rerender(
      <TestContainer
        rows={[
          "New row",
          "Row 1",
          "Row 2",
          "Row 3",
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );

    expect(list.scrollTop).toBe(0);
    expect(onRowChangeAbove).not.toHaveBeenCalled();
  });

  it("sticks the topmost element when adding new rows above", () => {
    const onRowChangeAbove = jest.fn();

    const { rerender, getByTestId } = render(
      <TestContainer
        rows={[
          "Row 1",
          "Row 2",
          "Row 3",
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );
    const list = getByTestId("scroll");
    expect(onRowChangeAbove).not.toHaveBeenCalled();

    // Scroll to half of row 3
    fireEvent.scroll(list, { target: { scrollTop: ROW_HEIGHT * 2.5 } });

    rerender(
      <TestContainer
        rows={[
          "New row 1",
          "New row 2",
          "Row 1",
          "Row 2",
          "Row 3",
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );

    // Expect row 3’s half to be still at the top of the screen
    expect(list.scrollTop).toBe(ROW_HEIGHT * 4.5);
    expect(onRowChangeAbove).toHaveBeenCalled();
  });

  it("sticks the topmost element when removing rows above", () => {
    const onRowChangeAbove = jest.fn();

    const { rerender, getByTestId } = render(
      <TestContainer
        rows={[
          "Row 1",
          "Row 2",
          "Row 3",
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );
    const list = getByTestId("scroll");
    expect(onRowChangeAbove).not.toHaveBeenCalled();

    // Scroll to half of row 3
    fireEvent.scroll(list, { target: { scrollTop: ROW_HEIGHT * 2.5 } });

    rerender(
      <TestContainer
        rows={["Row 3", "Row 4", "Row 5", "Row 6", "Row 7", "Row 8", "Row 9"]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );

    // Expect row 3’s half to be still at the top of the screen
    expect(list.scrollTop).toBe(ROW_HEIGHT * 0.5);
    expect(onRowChangeAbove).toHaveBeenCalled();
  });

  it("doesn’t change the element height when the topmost element is deleted", () => {
    const onRowChangeAbove = jest.fn();

    const { rerender, getByTestId } = render(
      <TestContainer
        rows={[
          "Row 1",
          "Row 2",
          "Row 3",
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );
    const list = getByTestId("scroll");

    // Scroll to half of row 3
    fireEvent.scroll(list, { target: { scrollTop: ROW_HEIGHT * 2.5 } });

    rerender(
      <TestContainer
        rows={[
          "Row 1",
          "Row 2",
          // (removed)
          "Row 4",
          "Row 5",
          "Row 6",
          "Row 7",
          "Row 8",
          "Row 9",
        ]}
        onRowChangeAbove={onRowChangeAbove}
      />,
    );

    // The scroll position hasn’t changed
    expect(list.scrollTop).toBe(ROW_HEIGHT * 2.5);
    expect(onRowChangeAbove).not.toHaveBeenCalled();
  });
});

function TestContainer({
  rows,
  onRowChangeAbove,
}: {
  rows: string[];
  onRowChangeAbove: () => void;
}) {
  const scrollRef = useRef<HTMLUListElement>(null);
  useMaintainScrollPositionOnChange(
    rows,
    scrollRef,
    (row) => row,
    ROW_HEIGHT,
    onRowChangeAbove,
  );

  return (
    <ul
      style={{ height: "50px", overflowY: "auto" }}
      data-testid="scroll"
      ref={scrollRef}
    >
      {rows.map((row) => (
        <li key={row} style={{ height: `${ROW_HEIGHT}px` }}>
          {row}
        </li>
      ))}
    </ul>
  );
}
