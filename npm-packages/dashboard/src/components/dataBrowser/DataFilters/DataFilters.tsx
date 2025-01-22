import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ExclamationTriangleIcon,
} from "@radix-ui/react-icons";
import { Button, Tooltip, SchemaJson, Sheet } from "dashboard-common";
import { GenericDocument } from "convex/server";
import { convexToJson, jsonToConvex, ValidatorJSON } from "convex/values";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useMap } from "react-use";
import {
  Filter,
  FilterExpression,
  FilterValidationError,
  isTypeFilterOp,
} from "system-udfs/convex/_system/frontend/lib/filters";
import isEqual from "lodash/isEqual";
import cloneDeep from "lodash/cloneDeep";
import { useFilterHistory } from "hooks/useTableFilters";
import { FilterEditor, FilterState } from "../FilterEditor/FilterEditor";
import {
  documentValidatorForTable,
  validatorForColumn,
} from "../Table/utils/validators";

export const filterMenuId = "filterMenu";
export function DataFilters({
  defaultDocument,
  tableName,
  tableFields,
  componentId,
  filters,
  onChangeFilters,
  dataFetchErrors,
  draftFilters,
  setDraftFilters,
  activeSchema,
}: {
  defaultDocument: GenericDocument;
  tableName: string;
  tableFields: string[];
  componentId: string | null;
  filters?: FilterExpression;
  onChangeFilters(next: FilterExpression): void;
  dataFetchErrors?: FilterValidationError[];
  draftFilters?: FilterExpression;
  setDraftFilters(next: FilterExpression): void;
  activeSchema: SchemaJson | null;
}) {
  const [invalidFilters, { set: setInvalidFilters }] = useMap();

  const isDirty = !isEqual(filters, draftFilters);
  const hasInvalidFilters =
    Object.values(invalidFilters).filter((v) => v !== undefined).length > 0;

  const shownFilters = useMemo(
    () =>
      (draftFilters?.clauses.length ?? 0) === 0
        ? { clauses: [generateNewFilter()] }
        : draftFilters!,
    [draftFilters],
  );

  const onChangeFilter = useCallback(
    (filter: FilterState, idx: number) => {
      const newFilters = cloneDeep(shownFilters);

      // Convert the FilterState to a Filter
      let newFilter: Filter;
      if (filter.op === "type" || filter.op === "notype") {
        // Type filters are special because they have a value that is not a JSONValue.
        newFilter = filter;
      } else if (filter.op === "anyOf" || filter.op === "noneOf") {
        // Return an incomplete filter for either of these operators
        newFilter = { op: "eq" };
      } else {
        newFilter = {
          op: filter.op,
          id: shownFilters.clauses[idx].id,
          field: filter.field,
          value:
            filter.value !== undefined
              ? convexToJson(filter.value)
              : filter.value,
        };
      }

      newFilters.clauses[idx] = newFilter;
      setDraftFilters(newFilters);
    },
    [shownFilters, setDraftFilters],
  );

  const onDeleteFilter = useCallback(
    (idx: number) => {
      const newFilters = {
        ...shownFilters,
        clauses: [
          ...shownFilters.clauses.slice(0, idx),
          ...shownFilters.clauses.slice(idx + 1),
        ],
      };
      if (newFilters.clauses.length === 0) {
        onChangeFilters({ clauses: [] });
      } else {
        setDraftFilters(newFilters);
      }
    },
    [shownFilters, setDraftFilters, onChangeFilters],
  );

  const onAddFilter = (idx: number) => {
    const newFilters = {
      ...shownFilters,
      clauses: [
        ...shownFilters.clauses.slice(0, idx),
        generateNewFilter(),
        ...shownFilters.clauses.slice(idx),
      ],
    };
    setDraftFilters(newFilters);
  };

  const onError = useCallback(
    (idx: number, errors: string[]) => {
      setInvalidFilters(idx, errors.length || undefined);
    },
    [setInvalidFilters],
  );

  const { filterHistory } = useFilterHistory(tableName, componentId);
  const [currentIdx, setCurrentIdx] = useState(0);
  useEffect(() => {
    setCurrentIdx(0);
  }, [filterHistory]);

  const documentValidator = activeSchema
    ? documentValidatorForTable(activeSchema, tableName)
    : undefined;

  return (
    <Sheet className="ml-auto w-fit p-3" padding={false}>
      <div className="mb-2 flex w-full items-center justify-between gap-2">
        <h5>Filters</h5>
        <div className="flex gap-2">
          <Button
            size="xs"
            variant="neutral"
            icon={<ChevronLeftIcon />}
            tip="Previous Filters"
            disabled={currentIdx + 1 >= filterHistory.length}
            onClick={() => {
              setCurrentIdx(currentIdx + 1);
              setDraftFilters(filterHistory[currentIdx + 1]);
            }}
          />
          <Button
            size="xs"
            variant="neutral"
            icon={<ChevronRightIcon />}
            tip="Next Filters"
            disabled={currentIdx <= 0}
            onClick={() => {
              setCurrentIdx(currentIdx - 1);
              setDraftFilters(filterHistory[currentIdx - 1]);
            }}
          />
        </div>
      </div>
      <form
        className="flex flex-col gap-2"
        id={filterMenuId}
        data-testid="filterMenu"
        onSubmit={(e) => {
          e.preventDefault();
          onChangeFilters(draftFilters || { clauses: [] });
        }}
        key={currentIdx}
      >
        <div className="flex flex-col gap-1.5 px-2 scrollbar">
          {shownFilters.clauses.map((clause, idx) => (
            <FilterItem
              key={clause.id}
              idx={idx}
              autoFocusValueEditor={idx === shownFilters.clauses.length - 1}
              tableName={tableName}
              fields={tableFields}
              clause={clause}
              defaultDocument={defaultDocument}
              onChangeFilter={onChangeFilter}
              onAddFilter={onAddFilter}
              onDeleteFilter={onDeleteFilter}
              onApplyFilters={() =>
                onChangeFilters(draftFilters || { clauses: [] })
              }
              onError={onError}
              error={
                invalidFilters[idx]
                  ? "Invalid syntax for filter value."
                  : dataFetchErrors?.find((e) => e.filter === idx)?.error
              }
              documentValidator={documentValidator}
              shouldSurfaceValidatorErrors={activeSchema?.schemaValidation}
            />
          ))}
        </div>
        <div className="ml-auto flex items-center justify-between gap-1">
          {dataFetchErrors && dataFetchErrors.length > 0 && (
            <p
              className="h-4 break-words text-xs text-content-errorSecondary"
              role="alert"
            >
              These filters are invalid, fix or remove invalid filters to
              continue.
            </p>
          )}
          <Tooltip
            tip={
              hasInvalidFilters
                ? "Fix the errors above to apply your filters."
                : !isDirty
                  ? "Update the filters to apply changes."
                  : undefined
            }
            wrapsButton
          >
            <Button
              type="submit"
              disabled={hasInvalidFilters || !isDirty}
              size="xs"
              data-testid="apply-filters"
            >
              Apply filters
            </Button>
          </Tooltip>
        </div>
      </form>
    </Sheet>
  );
}

function generateNewFilter(): Filter {
  return {
    // Allocate an ID for the new clause on the client side
    // To be used for the key prop in the FilterEditor
    id: Math.random().toString(),
    field: "_id",
    op: "eq",
    value: "",
  };
}

function FilterItem({
  idx,
  fields,
  defaultDocument,
  clause,
  onChangeFilter,
  onAddFilter,
  onDeleteFilter,
  onApplyFilters,
  onError,
  error,
  autoFocusValueEditor = false,
  documentValidator,
  shouldSurfaceValidatorErrors,
  tableName,
}: {
  idx: number;
  fields: string[];
  defaultDocument: GenericDocument;
  clause: Filter;
  onAddFilter(idx: number): void;
  onChangeFilter(filter: FilterState, idx: number): void;
  onDeleteFilter(idx: number): void;
  onApplyFilters(): void;
  onError(idx: number, errors: string[]): void;
  error?: string;
  autoFocusValueEditor?: boolean;
  documentValidator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
  tableName: string;
}) {
  const onChange = useCallback(
    (filter: FilterState) => {
      onChangeFilter(filter, idx);
    },
    [idx, onChangeFilter],
  );
  const onDelete = useCallback(
    () => onDeleteFilter(idx),
    [idx, onDeleteFilter],
  );
  const onAdd = useCallback(() => onAddFilter(idx + 1), [idx, onAddFilter]);

  const handleError = useCallback(
    (errors: string[]) => onError(idx, errors),
    [idx, onError],
  );

  // Convert the Filter into a FilterState
  const defaultValue = useMemo<FilterState>(() => {
    // Return an incomplete filter for either of these operators
    if (clause.op === "anyOf" || clause.op === "noneOf") {
      return { op: "eq" };
    }
    // Type filters are special because they have a value that is not a JSONValue.
    if (isTypeFilterOp(clause.op)) {
      return clause;
    }
    let value = null;
    if (clause.value !== undefined) {
      try {
        value = jsonToConvex(clause.value);
      } catch (e) {
        // couldn't parse the value, so leave it as null
      }
    }
    return {
      field: clause.field,
      op: clause.op,
      value,
    };
  }, [clause]);

  const validator = documentValidator
    ? validatorForFilterField(documentValidator, tableName, clause.field)
    : undefined;
  return (
    <div className="ml-auto flex items-start gap-2" key={idx}>
      {error ? (
        <Tooltip tip={error}>
          <ExclamationTriangleIcon className="mt-2.5 h-5 w-5 text-content-errorSecondary" />
        </Tooltip>
      ) : (
        <div className="w-5" />
      )}
      <FilterEditor
        id={clause.id || idx.toString()}
        fields={fields}
        defaultDocument={defaultDocument}
        defaultValue={defaultValue}
        onChange={onChange}
        onError={handleError}
        onAdd={onAdd}
        onDelete={onDelete}
        onApplyFilters={onApplyFilters}
        autoFocusValueEditor={autoFocusValueEditor}
        validator={validator}
        shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
      />
    </div>
  );
}

const validatorForFilterField = (
  documentValidator: SchemaJson["tables"][0]["documentType"],
  tableName: string,
  fieldName?: string,
): ValidatorJSON | undefined => {
  if (!documentValidator || fieldName === undefined) {
    return undefined;
  }

  switch (fieldName) {
    case "_id":
      return { type: "id", tableName };
    case "_creationTime":
      return { type: "number" };
    default:
      return validatorForColumn(documentValidator, fieldName);
  }
};
