import React, { useCallback, useId } from "react";
import {
  FilterByIndex,
  FilterByIndexRange,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Tooltip } from "@ui/Tooltip";
import { TextInput } from "@ui/TextInput";
import { MagnifyingGlassIcon } from "@radix-ui/react-icons";

export type IndexFilterState = FilterByIndex | FilterByIndexRange;

export function SearchValueEditor({
  field,
  value,
  onChange,
  onApplyFilters,
}: {
  field: string;
  value: string;
  onChange(newValue: string): void;
  onApplyFilters(): void;
}) {
  const inputId = useId();

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange],
  );

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (event.key === "Enter") {
        onApplyFilters();
      }
    },
    [onApplyFilters],
  );

  return (
    <div className="flex min-w-0 items-center gap-2">
      <div className="flex min-w-0 grow">
        {/* Field name display */}
        <Tooltip tip="You cannot edit this field because it is a part of the definition of the selected index.">
          <label
            htmlFor={inputId}
            className="flex h-full max-w-[7rem] min-w-[1rem] cursor-not-allowed items-center truncate rounded-l border border-r-0 bg-background-secondary px-2 py-1 text-xs"
          >
            {field}
          </label>
        </Tooltip>

        <TextInput
          id={inputId}
          value={value}
          onChange={handleChange}
          labelHidden
          className="rounded-l-none py-1 text-xs"
          onKeyDown={handleKeyDown}
          leftAddon={<MagnifyingGlassIcon className="text-content-tertiary" />}
        />
      </div>
    </div>
  );
}
