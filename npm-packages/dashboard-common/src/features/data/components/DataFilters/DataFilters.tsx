import {
  ArrowLeftIcon,
  ArrowRightIcon,
  CheckIcon,
  ExclamationTriangleIcon,
  PlusIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { GenericDocument } from "convex/server";
import {
  Filter,
  FilterExpression,
  FilterValidationError,
} from "system-udfs/convex/_system/frontend/lib/filters";
import {
  FilterEditor,
  FilterState,
} from "@common/features/data/components/FilterEditor/FilterEditor";
import { SchemaJson } from "@common/lib/format";
import { Button } from "@common/elements/Button";
import { Tooltip } from "@common/elements/Tooltip";
import {
  FilterButton,
  filterMenuId,
} from "@common/features/data/components/DataFilters/FilterButton";
import { ValidatorJSON, convexToJson } from "convex/values";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useMap } from "react-use";
import isEqual from "lodash/isEqual";
import cloneDeep from "lodash/cloneDeep";
import {
  documentValidatorForTable,
  validatorForColumn,
} from "@common/features/data/components/Table/utils/validators";
import { useFilterHistory } from "@common/features/data/lib/useTableFilters";
import { cn } from "@common/lib/cn";

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
  numRows,
  numRowsLoaded,
  hasFilters,
  showFilters,
  setShowFilters,
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
  numRows?: number;
  numRowsLoaded: number;
  hasFilters: boolean;
  showFilters: boolean;
  setShowFilters: React.Dispatch<React.SetStateAction<boolean>>;
}) {
  const {
    isDirty,
    hasInvalidFilters,
    shownFilters,
    onChangeFilter,
    onDeleteFilter,
    onAddFilter,
    onError,
    filterHistory,
    currentIdx,
    setCurrentIdx,
    documentValidator,
    invalidFilters,
  } = useDataFilters({
    tableName,
    componentId,
    filters,
    onChangeFilters,
    draftFilters,
    setDraftFilters,
    activeSchema,
  });

  const numRowsWeKnowOf = hasFilters ? numRowsLoaded : numRows;

  return (
    <form
      className="flex w-full flex-col gap-2 rounded-t border border-b-0 bg-background-secondary/50 p-2"
      id={filterMenuId}
      data-testid="filterMenu"
      onSubmit={(e) => {
        e.preventDefault();
        onChangeFilters(draftFilters || { clauses: [] });
      }}
      key={currentIdx}
    >
      <div className="flex flex-col">
        <div className="flex justify-between gap-2">
          <div className="flex items-center">
            <div className="flex w-full rounded bg-background-secondary">
              <div className="flex items-center">
                <Button
                  size="xs"
                  variant="neutral"
                  className={cn(
                    "rounded-r-none border border-r-0",
                    showFilters && "rounded-b-none border-b-0",
                  )}
                  icon={<ArrowLeftIcon className="my-[1px]" />}
                  inline
                  tip="Previous Filters"
                  disabled={currentIdx + 1 >= filterHistory.length}
                  onClick={() => {
                    setShowFilters(true);
                    setCurrentIdx(currentIdx + 1);
                    setDraftFilters(filterHistory[currentIdx + 1]);
                  }}
                />
                <Button
                  size="xs"
                  variant="neutral"
                  className={cn(
                    "rounded-none border border-x-0",
                    showFilters && "border-b-0",
                  )}
                  icon={<ArrowRightIcon className="my-[1px]" />}
                  tip="Next Filters"
                  inline
                  disabled={currentIdx <= 0}
                  onClick={() => {
                    setShowFilters(true);
                    setCurrentIdx(currentIdx - 1);
                    setDraftFilters(filterHistory[currentIdx - 1]);
                  }}
                />
              </div>
              <FilterButton
                filters={filters}
                onClick={() => {
                  if (!showFilters && shownFilters.clauses.length === 0) {
                    onAddFilter(0);
                  }
                  setShowFilters(!showFilters);
                }}
                open={showFilters}
              />
            </div>
          </div>
          <div className="flex gap-2">
            {numRowsWeKnowOf !== undefined && (
              <div
                className={cn(
                  "flex items-center gap-1",
                  "text-xs whitespace-nowrap",
                )}
              >
                <span className="font-semibold">
                  {numRowsWeKnowOf.toLocaleString()}{" "}
                </span>
                {numRowsWeKnowOf === 1 ? "document" : "documents"}{" "}
                {hasFilters && (
                  <>
                    {numRowsWeKnowOf !== numRows && `loaded`}
                    <Tooltip
                      tip="Filtered results are paginated and more documents will be loaded as you scroll."
                      side="right"
                    >
                      <QuestionMarkCircledIcon />
                    </Tooltip>
                  </>
                )}
              </div>
            )}
          </div>
        </div>
        {showFilters && (
          <div className="w-full animate-fadeInFromLoading">
            <div className="flex w-full flex-col gap-1 overflow-x-auto rounded rounded-tl-none border bg-background-secondary p-2 pb-2.5 scrollbar">
              {shownFilters.clauses.map((clause, idx) => (
                <FilterItem
                  key={clause.id}
                  idx={idx}
                  autoFocusValueEditor={idx === shownFilters.clauses.length - 1}
                  fields={tableFields}
                  clause={clause}
                  defaultDocument={defaultDocument}
                  onChangeFilter={onChangeFilter}
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
                  documentValidator={documentValidator(clause.field)}
                  shouldSurfaceValidatorErrors={activeSchema?.schemaValidation}
                />
              ))}
              <div className="mt-1 flex items-center gap-1">
                <Button
                  variant="neutral"
                  size="xs"
                  className="text-xs"
                  icon={<PlusIcon />}
                  onClick={() => onAddFilter(shownFilters.clauses.length)}
                >
                  Add filter
                </Button>
                {isDirty ? (
                  <Button
                    type="submit"
                    tip={
                      hasInvalidFilters
                        ? "Fix the errors above to apply your filters."
                        : undefined
                    }
                    disabled={hasInvalidFilters || !isDirty}
                    size="xs"
                    data-testid="apply-filters"
                    className="text-xs"
                  >
                    Apply Filters
                  </Button>
                ) : (
                  <p className="ml-1 flex gap-0.5 text-xs font-medium text-content-secondary">
                    <CheckIcon />
                    Filters applied
                  </p>
                )}
                {dataFetchErrors && dataFetchErrors.length > 0 && (
                  <p
                    className="h-4 break-words text-xs text-content-errorSecondary"
                    role="alert"
                  >
                    These filters are invalid, fix or remove invalid filters to
                    continue.
                  </p>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </form>
  );
}

function FilterItem({
  idx,
  fields,
  defaultDocument,
  clause,
  onChangeFilter,
  onDeleteFilter,
  onApplyFilters,
  onError,
  error,
  autoFocusValueEditor = false,
  documentValidator,
  shouldSurfaceValidatorErrors,
}: {
  idx: number;
  fields: string[];
  defaultDocument: GenericDocument;
  clause: Filter;
  onChangeFilter(filter: FilterState, idx: number): void;
  onDeleteFilter(idx: number): void;
  onApplyFilters(): void;
  onError(idx: number, errors: string[]): void;
  error?: string;
  autoFocusValueEditor?: boolean;
  documentValidator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
}) {
  return (
    <div className="flex items-start gap-2" key={idx}>
      <FilterEditor
        id={clause.id || idx.toString()}
        fields={fields}
        defaultDocument={defaultDocument}
        defaultValue={clause}
        onChange={(filter) => onChangeFilter(filter, idx)}
        onError={(errors) => onError(idx, errors)}
        onDelete={() => onDeleteFilter(idx)}
        onApplyFilters={onApplyFilters}
        autoFocusValueEditor={autoFocusValueEditor}
        validator={documentValidator}
        shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
      />
      {error && (
        <Tooltip tip={error}>
          <ExclamationTriangleIcon className="mt-1.5 size-4 text-content-errorSecondary" />
        </Tooltip>
      )}
    </div>
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

function validatorForFilterField(
  documentValidator: SchemaJson["tables"][0]["documentType"],
  tableName: string,
  fieldName?: string,
): ValidatorJSON | undefined {
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
}

function useDataFilters({
  tableName,
  componentId,
  filters,
  onChangeFilters,
  draftFilters,
  setDraftFilters,
  activeSchema,
}: {
  tableName: string;
  componentId: string | null;
  filters?: FilterExpression;
  onChangeFilters(next: FilterExpression): void;
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
        ? { clauses: [] }
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

  const onAddFilter = useCallback(
    (idx: number) => {
      const newFilters = {
        ...shownFilters,
        clauses: [
          ...shownFilters.clauses.slice(0, idx),
          generateNewFilter(),
          ...shownFilters.clauses.slice(idx),
        ],
      };
      setDraftFilters(newFilters);
    },
    [shownFilters, setDraftFilters],
  );

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

  const getValidatorForField = useCallback(
    (fieldName?: string) =>
      documentValidator
        ? validatorForFilterField(documentValidator, tableName, fieldName)
        : undefined,
    [documentValidator, tableName],
  );

  return {
    isDirty,
    hasInvalidFilters,
    shownFilters,
    onChangeFilter,
    onDeleteFilter,
    onError,
    onAddFilter,
    invalidFilters,
    filterHistory,
    currentIdx,
    setCurrentIdx,
    documentValidator: getValidatorForField,
  };
}
