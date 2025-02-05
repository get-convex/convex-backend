import { BackspaceIcon } from "@heroicons/react/20/solid";
import { PlusIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { GenericDocument } from "convex/server";
import { ValidatorJSON, Value } from "convex/values";
import isEqual from "lodash/isEqual";
import React, { useCallback, useReducer } from "react";
import {
  Filter,
  FilterByBuiltin,
  FilterByOr,
  FilterByType,
  TypeFilterValue,
  isTypeFilterOp,
  typeOf,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Combobox, type Option } from "@common/elements/Combobox";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";
import { Button } from "@common/elements/Button";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";

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
} & (
  | { op: FilterByBuiltin["op"]; value?: Value }
  | { op: FilterByType["op"]; value?: TypeFilterValue }
  | { op: FilterByOr["op"]; value?: Value[] }
);

export type FilterEditorProps = {
  fields: string[];
  onChange(filter: FilterState): void;
  onAdd(): void;
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
  onAdd,
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

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-1.5">
        <div className="flex gap-1.5">
          <Combobox
            label="Select filter field"
            buttonClasses="w-[10rem]"
            searchPlaceholder="Search fields..."
            options={fields.map((field) => ({ value: field, label: field }))}
            selectedOption={state.field}
            setSelectedOption={selectField(
              state,
              dispatch,
              defaultDocument,
              forceRerender,
            )}
            // If we only have two fields, it might be caused by the shapes computation returning
            // an “any” type. We allow the user to type arbitrary fields name in this case.
            allowCustomValue={fields.length === ["_id", "_creationTime"].length}
          />
          <Combobox
            label="Select filter operator"
            searchPlaceholder="Search operators..."
            buttonClasses="w-[8rem]"
            options={operatorOptions}
            selectedOption={state.op}
            setSelectedOption={selectOperator(
              state,
              dispatch,
              defaultDocument,
              () => {
                onError([]); // Clear any error state from the previous operator values.
                forceRerender(); // Force a re-render to clear any error state from the monaco editor.
              },
            )}
          />
        </div>
        <div className="flex items-center gap-1.5">
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
            size="sm"
            variant="neutral"
            inline
            onClick={onDelete}
            aria-label={`Delete filter ${id}`}
            icon={<BackspaceIcon className="h-4 w-4" />}
          />
          <Button
            size="sm"
            variant="neutral"
            inline
            onClick={onAdd}
            aria-label={`Add filter after ${id}`}
            icon={<PlusIcon />}
          />
        </div>
      </div>
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

  return (
    <div
      className={classNames(
        "flex w-[18rem]",
        // Combobox for type filter and date picker has it's own border.
        !isTypeFilterOp(state.op) && !isDatepicker && "border rounded",
      )}
    >
      {isTypeFilterOp(state.op) ? (
        <Combobox
          searchPlaceholder="Search types..."
          label="Select type value"
          buttonClasses="w-[18rem]"
          optionsWidth="full"
          options={typeOptions}
          selectedOption={state.value}
          setSelectedOption={(option) => {
            dispatch({ value: option });
          }}
        />
      ) : isDatepicker ? (
        <DateTimePicker
          inputClassName="w-[18rem]"
          date={
            // Convert to date from Unix timestamp, defaulting to now.
            typeof state.value === "number" ? new Date(state.value) : new Date()
          }
          onChange={(date) => {
            // Change back to Unix timestamp.
            dispatch({ value: date?.getTime() });
          }}
        />
      ) : (
        <ObjectEditor
          key={objectEditorKey}
          className="border-none pl-3"
          disableFolding
          defaultValue={state.value}
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
      dispatch({ field: option, op: "lte", value: Date.now() });
    } else {
      // Switching to any other field should set the value to the default value for that field.
      let newValue = state.value !== undefined ? state.value : null;
      if (isTypeFilterOp(state.op)) {
        newValue =
          option in defaultDocument ? typeOf(defaultDocument[option]) : "unset";
      } else {
        // If we're switching to a builtin filter, try to use the default value from the document.
        const defaultFilterValue =
          option in defaultDocument ? defaultDocument[option] : null;

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
