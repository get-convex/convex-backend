import { RefObject, useCallback, useEffect, useState } from "react";
import { useWindowSize } from "react-use";
import { FixedSizeList } from "react-window";
import { useTableDensity } from "@common/features/data/lib/useTableDensity";

const MIN_SCROLLBAR_SIZE = 12;

// The handlers returned by this hook were mostly copied/inspired by:
// https://www.thisdot.co/blog/creating-custom-scrollbars-with-react
function useScrollbar(
  totalRowCount: number,
  outerRef: RefObject<HTMLElement>,
  listRef: RefObject<FixedSizeList>,
) {
  const { densityValues } = useTableDensity();
  // Recompute scrollbar size when window is resize by forcing a rerender.
  useWindowSize();

  const totalRowHeight = totalRowCount * densityValues.height;

  const { current: outerEl } = outerRef;
  const { current: listEl } = listRef;

  const scrollbarHeight =
    !outerEl || totalRowHeight < outerEl.offsetHeight
      ? 0
      : (outerEl.offsetHeight / totalRowHeight) * outerEl.offsetHeight;

  const [scrollStartPosition, setScrollStartPosition] = useState<number | null>(
    null,
  );
  const [initialScrollTop, setInitialScrollTop] = useState<number>(0);
  const [isDragging, setIsDragging] = useState(false);

  const handleTrackClick = useCallback(
    (e: React.MouseEvent) => {
      if (outerEl && listEl) {
        // First, figure out where we clicked
        const { clientY } = e;
        // Next, figure out the distance between the top of the track and the top of the viewport
        const target = e.target as HTMLDivElement;
        const rect = target.getBoundingClientRect();
        const trackTop = rect.top;
        // We want the middle of the thumb to jump to where we clicked, so we subtract half the thumb's height to offset the position
        const thumbOffset = -(scrollbarHeight / 2);
        // Find the ratio of the new position to the total content length using the thumb and track values...
        const clickRatio =
          (clientY - trackTop + thumbOffset) / outerEl.clientHeight;
        // ...so that you can compute where the content should scroll to.
        const scrollAmount = Math.floor(
          clickRatio * totalRowCount * densityValues.height,
        );
        // And finally, scroll to the new position!
        listEl.scrollTo(scrollAmount);
      }
    },
    [outerEl, listEl, scrollbarHeight, totalRowCount, densityValues.height],
  );
  const handleThumbMousedown = useCallback(
    (e: MouseEvent) => {
      setScrollStartPosition(e.clientY);
      if (outerEl) setInitialScrollTop(outerEl.scrollTop);
      setIsDragging(true);
    },
    [outerEl],
  );

  const handleThumbMouseup = useCallback(() => {
    if (isDragging) {
      setIsDragging(false);
    }
  }, [isDragging]);

  const handleThumbMousemove = useCallback(
    (e: MouseEvent) => {
      if (isDragging && outerEl && listEl && scrollStartPosition) {
        e.preventDefault();
        e.stopPropagation();
        const {
          scrollHeight: contentScrollHeight,
          offsetHeight: contentOffsetHeight,
        } = outerEl;

        // Subtract the current mouse y position from where you started to get the pixel difference in mouse position. Multiply by ratio of visible content height to thumb height to scale up the difference for content scrolling.
        const deltaY =
          (e.clientY - scrollStartPosition) *
          (contentOffsetHeight / scrollbarHeight);
        const newScrollTop = Math.min(
          initialScrollTop + deltaY,
          contentScrollHeight - contentOffsetHeight,
        );

        listEl?.scrollTo(newScrollTop);
      }
    },
    [
      isDragging,
      scrollStartPosition,
      scrollbarHeight,
      outerEl,
      listEl,
      initialScrollTop,
    ],
  );

  // Listen for mouse events to handle scrolling by dragging the thumb
  useEffect(() => {
    document.addEventListener("mousemove", handleThumbMousemove);
    document.addEventListener("mouseup", handleThumbMouseup);
    document.addEventListener("mouseleave", handleThumbMouseup);
    return () => {
      document.removeEventListener("mousemove", handleThumbMousemove);
      document.removeEventListener("mouseup", handleThumbMouseup);
      document.removeEventListener("mouseleave", handleThumbMouseup);
    };
  }, [handleThumbMousemove, handleThumbMouseup]);

  return {
    listRef,
    outerRef,
    scrollbarHeight,
    scrollbarTop: outerEl
      ? // Use Math.min and Math.max to make sure the scrollbar doesn't go out of bounds.
        Math.min(
          Math.max(
            (outerEl.scrollTop / totalRowHeight) * outerEl.offsetHeight,
            0,
          ),
          outerEl.offsetHeight - scrollbarHeight,
        )
      : 0,
    handleTrackClick,
    handleThumbMousedown,
  };
}

export function TableScrollbar({
  totalRowCount,
  outerRef,
  listRef,
}: {
  totalRowCount?: number;
  outerRef: RefObject<HTMLElement>;
  listRef: RefObject<FixedSizeList>;
}) {
  const {
    scrollbarHeight,
    scrollbarTop,
    handleTrackClick,
    handleThumbMousedown,
  } = useScrollbar(totalRowCount || 0, outerRef, listRef);

  // Create a React handler from a native event handler, just for the types.
  const handleReactThumbMousedown = useCallback(
    (e: React.MouseEvent) => handleThumbMousedown(e.nativeEvent),
    [handleThumbMousedown],
  );

  const { densityValues } = useTableDensity();
  return scrollbarHeight >= 0 ? (
    <div
      className="absolute right-0 w-2"
      role="scrollbar"
      aria-controls="dataTable"
      aria-valuenow={scrollbarTop}
      style={{
        height: `${outerRef.current?.offsetHeight}px`,
        top: densityValues.height,
      }}
    >
      {/* eslint-disable  */}
      {/* I have no clue how to properly do a11y for this scrollbar, 
            but it seems to work well for scrollbars */}
      <div onClick={handleTrackClick} className="fixed h-full w-2" />
      <div
        style={{
          height:
            scrollbarHeight === 0
              ? // No scrollbar needed
                0
              : // Use the calculated size of the scrollbar, or the min size,
                // whichever is greater.
                Math.max(scrollbarHeight, MIN_SCROLLBAR_SIZE),
          marginTop:
            scrollbarHeight < MIN_SCROLLBAR_SIZE
              ? // If the scrollbar is using the minimum size, add some margin
                // to the top so it snaps to the top and bottom of the table.
                Math.max(scrollbarTop - (MIN_SCROLLBAR_SIZE + 1), 0)
              : scrollbarTop,
        }}
        onMouseDown={handleReactThumbMousedown}
        className={`fixed w-2 bg-neutral-1 dark:bg-neutral-8`}
      />
      {/* eslint-enable */}
    </div>
  ) : null;
}
