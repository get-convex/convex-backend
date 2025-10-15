import classNames from "classnames";
import { useEffect, useRef } from "react";

export function Checkbox({
  checked,
  className,
  onChange,
  onKeyDown,
  disabled = false,
  id = undefined,
}: {
  checked: boolean | "indeterminate";
  className?: string;
  onChange: React.EventHandler<React.SyntheticEvent<HTMLInputElement>>;
  onKeyDown?: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  disabled?: boolean;
  id?: string;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const isIndeterminate = checked === "indeterminate";
  useEffect(() => {
    if (inputRef.current !== null) {
      inputRef.current.indeterminate = isIndeterminate;
    }
  }, [isIndeterminate]);
  const checkedBool = checked === "indeterminate" ? false : checked;
  return (
    <input
      id={id}
      ref={inputRef}
      tabIndex={0}
      type="checkbox"
      className={classNames(
        "size-3.5 form-checkbox enabled:cursor-pointer rounded-sm disabled:opacity-50 disabled:bg-background-primary disabled:cursor-not-allowed enabled:hover:text-content-link enabled:hover:outline enabled:hover:outline-content-primary",
        "focus:outline-0 focus:ring-0",
        "bg-background-secondary ring-offset-background-secondary checked:bg-util-accent text-util-accent",
        className,
      )}
      onChange={onChange}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          onChange(event);
          return;
        }
        onKeyDown?.(event);
      }}
      disabled={disabled ?? false}
      checked={checkedBool}
      aria-checked={checkedBool}
      aria-label="Selected"
    />
  );
}
