import classNames from "classnames";
import { forwardRef, useId } from "react";
import { Checkbox } from "@ui/Checkbox";

type TableCheckboxProps = {
  checked: boolean;
  isSelectionAllNonExhaustive?: boolean;
  onToggle(): void;
  onToggleAdjacent(): void;
  className?: string;
  width?: string;
  onKeyDown?: (event: React.KeyboardEvent<HTMLInputElement>) => void;
};

export const TableCheckbox = forwardRef<HTMLLabelElement, TableCheckboxProps>(
  function TableCheckbox(
    {
      checked,
      isSelectionAllNonExhaustive = false,
      onToggle,
      onToggleAdjacent,
      onKeyDown,
      className = undefined,
      width,
    },
    ref,
  ) {
    const id = useId();

    return (
      <label
        ref={ref}
        htmlFor={id}
        aria-label="Select row or column"
        className={classNames(
          "flex items-center justify-center h-full cursor-pointer",
          className,
        )}
        style={{
          width,
        }}
      >
        <Checkbox
          id={id}
          className={
            checked && isSelectionAllNonExhaustive ? "opacity-50" : undefined
          }
          onKeyDown={onKeyDown}
          onChange={(event) => {
            // @ts-expect-error shiftKey will exist on change events triggered by the mouse
            if (event.nativeEvent.shiftKey) {
              onToggleAdjacent();
            } else {
              onToggle();
            }
          }}
          checked={checked}
        />
      </label>
    );
  },
);
