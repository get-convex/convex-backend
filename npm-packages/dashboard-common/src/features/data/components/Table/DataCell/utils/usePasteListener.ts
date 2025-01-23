import { Value } from "convex/values";
import { walkAst } from "dashboard-common";
import { useEffect } from "react";

export function usePasteListener(
  cellRef: React.RefObject<HTMLDivElement>,
  columnName: string,
  edit: (value: Value) => void,
  allowTopLevelUndefined: boolean = true,
) {
  useEffect(() => {
    const listener = (e: ClipboardEvent) => {
      if (
        columnName.startsWith("_") ||
        !cellRef.current?.contains(document.activeElement)
      ) {
        // Don't paste into this cell if it's a system field or if the cell isn't focused.
        // onPaste handler doesn't work because this component is not an input, so we use a global event listener.
        return;
      }

      const clipboardValue = e.clipboardData?.getData("text");
      if (clipboardValue === undefined) {
        return;
      }

      try {
        // Try to parse the Convex value out of the pasted value.
        const { value: parsedValue, errors } = walkAst(clipboardValue, {
          mode: "editField",
          allowTopLevelUndefined,
        });
        edit(errors.length > 0 ? clipboardValue : parsedValue);
      } catch (err) {
        // The error is likely a SyntaxError, which is thrown when the clipboard value is not valid JavaScript,
        // So paste the string value instead.
        edit(clipboardValue);
      }
    };

    window.addEventListener("paste", listener);

    return () => {
      window.removeEventListener("paste", listener);
    };
  }, [allowTopLevelUndefined, cellRef, columnName, edit]);
}
