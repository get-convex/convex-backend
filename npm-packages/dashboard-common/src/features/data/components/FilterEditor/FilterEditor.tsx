import { BackspaceIcon } from "@heroicons/react/24/outline";
import { GenericDocument } from "convex/server";
import { ValidatorJSON, Value } from "convex/values";
import isEqual from "lodash/isEqual";
import React, { useCallback, useReducer, useState } from "react";
import {
  Filter,
  FilterByBuiltin,
  FilterByOr,
  FilterByType,
  TypeFilterValue,
  isTypeFilterOp,
  typeOf,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Combobox, type Option } from "@ui/Combobox";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";
import { Button } from "@ui/Button";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";
import { Checkbox } from "@ui/Checkbox";
import { Tooltip } from "@ui/Tooltip";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/lib/values";
import { cn } from "@ui/cn";

export const operatorOptions: Readonly<
  Option<(FilterByType | FilterByBuiltin)["op"]>[]
> = [
  { value: "eq", label: "equals" },
  { value: "neq", label: "not equal" },
  { value: "gt", label: ">" },
  { value: "lt", label: "<" },
  { value: "gte", label: ">=" },
  { value: "lte", label: "<=" },
  { value: "type", label: "is type" },
  { value: "notype", label: "is not type" },
];

const typeOptions: Option<string>[] = [
  { value: "string", label: "string" },
  { value: "boolean", label: "boolean" },
  { value: "number", label: "number" },
  { value: "bigint", label: "bigint" },
  { value: "null", label: "null" },
  { value: "object", label: "object" },
  { value: "array", label: "array" },
  { value: "id", label: "id" },
  { value: "bytes", label: "bytes" },
  { value: "unset", label: "unset" },
];

export type FilterState = {
  field?: string;
  enabled?: boolean;
} & (
  | { op: FilterByBuiltin["op"]; value?: Value }
  | { op: FilterByType["op"]; value?: TypeFilterValue }
  | { op: FilterByOr["op"]; value?: Value[] }
);

export type FilterEditorProps = {
  fields: string[];
  onChange(filter: FilterState): void;
  onDelete(): void;
  onApplyFilters(): void;
  onError(errors: string[]): void;
  id?: string;
  defaultDocument: GenericDocument;
  defaultValue?: FilterState;
  autoFocusValueEditor?: boolean;
  validator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
};

export function FilterEditor({
  fields,
  onChange,
  onDelete,
  onError,
  onApplyFilters,
  id,
  defaultDocument,
  defaultValue,
  autoFocusValueEditor = false,
  validator,
  shouldSurfaceValidatorErrors,
}: FilterEditorProps) {
  const [state, dispatch] = useReducer(
    filterStateReducer,
    defaultValue || {
      field: undefined,
      op: "eq",
      value: null,
      enabled: true,
    },
  );

  function filterStateReducer(s: FilterState, action: Partial<FilterState>) {
    const newState = { ...s, ...action } as FilterState;
    !isEqual(s, newState) && onChange(newState);
    return newState;
  }

  const onChangeValue = useCallback((v?: Value) => {
    dispatch({ value: v });
  }, []);

  const [objectEditorKey, forceRerender] = useReducer((x) => x + 1, 0);

  const clearErrors = useCallback(() => {
    onError([]); // Clear any error state from the previous operator values.
    forceRerender(); // Force a re-render to clear any error state from the monaco editor.
  }, [onError, forceRerender]);

  return (
    <div className="flex min-w-0 grow">
      <Tooltip
        tip={state.enabled ? "Disable Filter" : "Enable Filter"}
        className="w-fit"
      >
        <Checkbox
          checked={state.enabled !== false}
          onChange={(e) => {
            dispatch({ enabled: e.currentTarget.checked });
          }}
          className="mr-2 self-center"
        />
      </Tooltip>
      <Combobox
        label="Select filter field"
        disabled={state.enabled === false}
        size="sm"
        optionsWidth="fixed"
        buttonClasses="min-w-fit w-fit max-w-[7rem] truncate"
        innerButtonClasses="rounded-r-none focus:border-r"
        searchPlaceholder="Search fields..."
        options={fields.map((field) => ({ value: field, label: field }))}
        selectedOption={state.field}
        setSelectedOption={selectField(
          state,
          dispatch,
          defaultDocument,
          forceRerender,
          clearErrors,
        )}
        // If we only have two fields, it might be caused by the shapes computation returning
        // an "any" type. We allow the user to type arbitrary fields name in this case.
        allowCustomValue={fields.length === ["_id", "_creationTime"].length}
      />
      <Combobox
        label="Select filter operator"
        searchPlaceholder="Search operators..."
        disabled={state.enabled === false}
        size="sm"
        optionsWidth="fixed"
        buttonClasses="w-fit"
        innerButtonClasses="w-fit rounded-r-none rounded-l-none ml-[-1px]"
        options={operatorOptions}
        selectedOption={state.op}
        setSelectedOption={selectOperator(
          state,
          dispatch,
          defaultDocument,
          clearErrors,
        )}
      />
      <ValueEditor
        {...{
          dispatch,
          state,
          id,
          objectEditorKey,
          onChangeValue,
          onApplyFilters,
          onError,
          autoFocus: autoFocusValueEditor,
          validator,
          shouldSurfaceValidatorErrors,
        }}
      />
      <Button
        size="xs"
        variant="neutral"
        onClick={onDelete}
        className="ml-[-1px] rounded-l-none"
        aria-label={`Delete filter ${id}`}
        icon={<BackspaceIcon className="size-4" />}
      />
    </div>
  );
}

function ValueEditor({
  dispatch,
  state,
  objectEditorKey,
  onChangeValue,
  onApplyFilters,
  onError,
  id,
  autoFocus,
  validator,
  shouldSurfaceValidatorErrors,
}: {
  dispatch: any;
  state: FilterState;
  objectEditorKey: number;
  onChangeValue: (v?: Value) => void;
  onApplyFilters: () => void;
  onError: (errors: string[]) => void;
  id?: string;
  autoFocus?: boolean;
  validator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
}) {
  const isDatepicker =
    state.field === "_creationTime" && typeof state.value === "number";

  const [innerText, setInnerText] = useState("");

  return (
    <div className="ml-[-1px] min-w-0 grow focus-within:z-20">
      {isTypeFilterOp(state.op) ? (
        <Combobox
          searchPlaceholder="Search types..."
          label="Select type value"
          disabled={state.enabled === false}
          buttonClasses="w-full rounded-l-none rounded-r-none"
          innerButtonClasses="w-full rounded-r-none rounded-l-none"
          size="sm"
          optionsWidth="fixed"
          options={typeOptions}
          selectedOption={state.value}
          setSelectedOption={(option) => {
            dispatch({ value: option });
          }}
        />
      ) : isDatepicker ? (
        <DateTimePicker
          date={
            // Convert to date from Unix timestamp, defaulting to now.
            typeof state.value === "number" ? new Date(state.value) : new Date()
          }
          disabled={state.enabled === false}
          onChange={(date) => {
            // Change back to Unix timestamp.
            dispatch({ value: date?.getTime() });
          }}
        />
      ) : (
        <>
          {innerText === "" && state.value === UNDEFINED_PLACEHOLDER && (
            <div
              className="pointer-events-none absolute z-50 font-mono text-xs italic text-content-secondary"
              data-testid="undefined-placeholder"
              style={{
                marginTop: "5px",
                marginLeft: "11px",
              }}
            >
              unset
            </div>
          )}
          <ObjectEditor
            key={objectEditorKey}
            className={cn(
              "min-w-4 rounded-none border focus-within:border focus-within:border-border-selected",
              state.enabled !== false && "border-x-transparent",
            )}
            editorClassname="px-2 py-1 mt-0 rounded-sm bg-background-secondary rounded-l-none rounded-r-none"
            allowTopLevelUndefined
            disabled={state.enabled === false}
            onChangeInnerText={setInnerText}
            size="sm"
            disableFolding
            defaultValue={
              state.value === UNDEFINED_PLACEHOLDER ? undefined : state.value
            }
            onChange={onChangeValue}
            onError={onError}
            path={`filter${id}`}
            autoFocus={autoFocus}
            disableFind
            saveAction={onApplyFilters}
            enterSaves
            mode="editField"
            validator={validator}
            shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
          />
        </>
      )}
    </div>
  );
}

const selectField =
  (
    state: FilterState,
    dispatch: React.Dispatch<Partial<FilterState>>,
    defaultDocument: GenericDocument,
    forceRerender: () => void,
    clearErrors: () => void,
  ) =>
  (option: string | null) => {
    if (option === null) {
      return;
    }
    if (option === "_id") {
      // Switching to the _id should always set the value to an empty ID.
      dispatch({
        field: option,
        op: "eq",
        value: "",
      });
    } else if (option === "_creationTime") {
      // Switching to creation time should always set the value to the current time.
      clearErrors();
      dispatch({ field: option, op: "lte", value: Date.now() });
      clearErrors();
    } else {
      // Switching to any other field should set the value to the default value for that field.
      let newValue = state.value !== undefined ? state.value : undefined;
      if (isTypeFilterOp(state.op)) {
        newValue =
          option in defaultDocument ? typeOf(defaultDocument[option]) : "unset";
      } else {
        // If we're switching to a builtin filter, try to use the default value from the document.
        const defaultFilterValue =
          option in defaultDocument ? defaultDocument[option] : undefined;

        // The value might already be of the correct type.
        if (typeOf(defaultFilterValue) !== typeOf(newValue)) {
          newValue = defaultFilterValue;
        }
      }
      dispatch({
        field: option,
        value: newValue,
      });
    }
    forceRerender();
  };

const selectOperator =
  (
    state: FilterState,
    dispatch: React.Dispatch<Partial<FilterState>>,
    defaultDocument: GenericDocument,
    clearErrors: () => void,
  ) =>
  (option: Filter["op"] | null) => {
    if (
      state.op === "anyOf" ||
      state.op === "noneOf" ||
      option === "anyOf" ||
      option === "noneOf" ||
      option === null
    ) {
      return;
    }
    // If we're switching to a type filter.
    if (isTypeFilterOp(option) && !isTypeFilterOp(state.op)) {
      const newValue = state.field
        ? // Try to use the default value from the document if we've filtered the field.
          typeOf(defaultDocument[state.field])
        : // Otherwise fall back to unset
          "unset";

      dispatch({
        op: option,
        value: newValue,
      });
      clearErrors();
      return;
    }

    // If we're switching from a type filter to a builtin filter, try to use the default value from the document.
    if (isTypeFilterOp(state.op) && !isTypeFilterOp(option)) {
      const newValue =
        state.field && state.field in defaultDocument
          ? defaultDocument[state.field]
          : state.field === "_id"
            ? ""
            : state.field === "_creationTime"
              ? Date.now()
              : null;
      clearErrors();
      dispatch({
        op: option,
        value: newValue,
      });
      return;
    }

    // Default case: not switching between types of filters.
    dispatch({ op: option });
  };
