import { ValidatorJSON, Value, convexToJson } from "convex/values";
import React, { useCallback } from "react";
import { Checkbox } from "@ui/Checkbox";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";
import { cn } from "@ui/cn";
import { Tooltip } from "@ui/Tooltip";
import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { SearchIndexFilterClause } from "system-udfs/convex/_system/frontend/lib/filters";
import { useIsMobileDevice } from "@ui/useIsMobileDevice";
import { ObjectEditorWithPlaceholder } from "./ObjectEditorWithPlaceholder";

export function SearchIndexFilterEditor({
  idx,
  field,
  error,
  onChange,
  onApplyFilters,
  onError,
  filter,
  autoFocusValueEditor = false,
  documentValidator,
  shouldSurfaceValidatorErrors,
}: {
  idx: number;
  field: string;
  error: string | undefined;
  onChange(filter: SearchIndexFilterClause, idx: number): void;
  onApplyFilters(): void;
  onError(idx: number, errors: string[]): void;
  filter: SearchIndexFilterClause;
  autoFocusValueEditor?: boolean;
  documentValidator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
}) {
  // Check if this is a _creationTime field
  const isCreationTimeField = field === "_creationTime";

  // Handle changes to the filter's enabled state
  const handleEnabledChange = useCallback(
    (event: React.SyntheticEvent<HTMLInputElement>) => {
      const enabled = (event.target as HTMLInputElement).checked;

      onChange({ ...filter, enabled }, idx);
    },
    [filter, idx, onChange],
  );

  // Handle changes to the filter's value
  const handleValueChange = useCallback(
    (value?: Value) => {
      onChange(
        {
          ...filter,
          value: value === undefined ? undefined : convexToJson(value),
        },
        idx,
      );
    },
    [filter, idx, onChange],
  );

  // Handle date change for _creationTime
  const handleDateChange = useCallback(
    (date: Date) => {
      const timestamp = date.getTime();
      onChange({ ...filter, value: timestamp }, idx);
    },
    [filter, idx, onChange],
  );

  // Handle errors from ObjectEditor
  const handleError = useCallback(
    (newErrors: string[]) => {
      onError(idx, newErrors);
    },
    [idx, onError],
  );

  // Helper to get timestamp value or default to current time
  const getTimestampValue = (value: any): Date => {
    if (typeof value === "number") {
      return new Date(value);
    }
    return new Date();
  };

  // On narrow screens the value editor drops to its own full-width line below
  // the field/operator so it has room to be usable.
  const isMobile = useIsMobileDevice();

  const checkbox = (
    <div className="flex items-center pr-2">
      <Checkbox
        checked={filter.enabled}
        onChange={handleEnabledChange}
        aria-label={`Enable filter on ${field}`}
      />
    </div>
  );

  const fieldPill = (
    <Tooltip
      tip={
        filter.enabled
          ? "You cannot edit this field because it is a part of the definition of the selected index."
          : undefined
      }
    >
      <div
        className={cn(
          "flex h-full max-w-48 min-w-4 cursor-not-allowed items-center truncate border bg-background-secondary px-2 py-1 text-xs",
          isMobile ? "rounded-sm" : "rounded-l",
          filter.enabled
            ? "bg-background-secondary"
            : "bg-background-tertiary text-content-secondary",
        )}
      >
        {field}
      </div>
    </Tooltip>
  );

  const operatorPill = (
    <Tooltip
      tip={
        filter.enabled &&
        "In an search index filter, index filters only support equality expressions."
      }
    >
      <div
        className={cn(
          "flex w-fit cursor-not-allowed items-center border px-1.5 py-1 text-xs",
          isMobile ? "rounded-sm" : "-ml-px",
          filter.enabled
            ? "bg-background-secondary"
            : "bg-background-tertiary text-content-secondary",
        )}
      >
        equals
      </div>
    </Tooltip>
  );

  const valueEditor = (
    <div className={cn("min-w-0", isMobile ? "w-full" : "-ml-px flex-1")}>
      {isCreationTimeField ? (
        <DateTimePicker
          date={getTimestampValue(filter.value)}
          onChange={handleDateChange}
          disabled={!filter.enabled}
          className={cn(
            "border p-1 text-xs",
            isMobile ? "w-full rounded-sm" : "rounded-r",
          )}
          aria-label="Creation time"
        />
      ) : (
        <ObjectEditorWithPlaceholder
          value={filter.value}
          onChangeHandler={handleValueChange}
          path={`searchIndexFilter${idx}-${field}`}
          autoFocus={autoFocusValueEditor}
          className="rounded-l-none rounded-r"
          isMobile={isMobile}
          enabled={filter.enabled}
          onApplyFilters={onApplyFilters}
          handleError={handleError}
          documentValidator={documentValidator}
          shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
        />
      )}
    </div>
  );

  const errorIcon = error && (
    <Tooltip tip={error}>
      <div className="ml-1 rounded-sm border bg-background-error p-1">
        <ExclamationTriangleIcon className="size-4 text-content-errorSecondary" />
      </div>
    </Tooltip>
  );

  if (isMobile) {
    return (
      <div className="flex min-w-0 flex-col gap-2">
        <div className="flex items-center gap-1">
          {checkbox}
          {fieldPill}
          {operatorPill}
          {errorIcon}
        </div>
        {valueEditor}
      </div>
    );
  }

  return (
    <div className="flex min-w-0 items-center gap-2">
      <div className="flex min-w-0 grow">
        {checkbox}
        {fieldPill}
        {operatorPill}
        {valueEditor}
        {errorIcon}
      </div>
    </div>
  );
}
