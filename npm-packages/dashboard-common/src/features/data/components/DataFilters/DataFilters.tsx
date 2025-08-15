import {
  ArrowLeftIcon,
  ArrowRightIcon,
  CheckIcon,
  ExclamationTriangleIcon,
  InfoCircledIcon,
  PlusIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { GenericDocument } from "convex/server";
import {
  Filter,
  FilterByIndex,
  FilterByIndexRange,
  FilterExpression,
  FilterValidationError,
} from "system-udfs/convex/_system/frontend/lib/filters";
import {
  FilterEditor,
  FilterState,
} from "@common/features/data/components/FilterEditor/FilterEditor";
import { SchemaJson } from "@common/lib/format";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import {
  FilterButton,
  filterMenuId,
} from "@common/features/data/components/DataFilters/FilterButton";
import { ValidatorJSON, convexToJson } from "convex/values";
import { useCallback, useContext, useEffect, useMemo, useState } from "react";
import { useMap } from "react-use";
import isEqual from "lodash/isEqual";
import cloneDeep from "lodash/cloneDeep";
import {
  documentValidatorForTable,
  validatorForColumn,
} from "@common/features/data/components/Table/utils/validators";
import {
  useFilterHistory,
  useTableFilters,
} from "@common/features/data/lib/useTableFilters";
import { cn } from "@ui/cn";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useNents } from "@common/lib/useNents";
import { useQuery } from "convex/react";
import { api } from "system-udfs/convex/_generated/api";
import { Index } from "@common/features/data/lib/api";
import { IndexFilterState } from "./IndexFilterEditor";
import { IndexFilters, getDefaultIndex } from "./IndexFilters";

export function DataFilters({
  defaultDocument,
  tableName,
  tableFields,
  componentId,
  filters,
  onFiltersChange,
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
  onFiltersChange(next: FilterExpression): void;
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
  const { selectedNent } = useNents();
  const indexes =
    (useQuery(api._system.frontend.indexes.default, {
      tableName,
      tableNamespace: selectedNent?.id ?? null,
    }) satisfies undefined | null | Index[]) ?? undefined;
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
    invalidFilters,
    onChangeOrder,
    getValidatorForField,
    onChangeIndexFilter,
    applyFiltersWithHistory,
  } = useDataFilters({
    tableName,
    componentId,
    filters,
    onFiltersChange,
    draftFilters,
    setDraftFilters,
    activeSchema,
  });

  const numRowsWeKnowOf = hasFilters ? numRowsLoaded : numRows;

  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();

  return (
    <form
      className="flex w-full flex-col gap-2 rounded-t-lg border border-b-0 bg-background-secondary/50 p-2"
      id={filterMenuId}
      data-testid="filterMenu"
      onSubmit={(e) => {
        e.preventDefault();
        if (hasInvalidFilters) {
          return;
        }
        log("apply filters", {
          hasIndexFilters:
            (shownFilters.index?.clauses || []).filter((c) => c.enabled)
              .length > 0,
          hasOtherFilters:
            shownFilters.clauses.filter((c) => c.enabled !== false).length > 0,
        });
        onFiltersChange(
          draftFilters || {
            clauses: [],
            index: undefined,
          },
        );
      }}
      key={currentIdx}
    >
      <div className="flex flex-col">
        <div className="flex justify-between gap-2">
          <div className="flex items-center">
            <div
              className={cn(
                "flex w-full rounded-lg border bg-background-secondary",
                showFilters && "rounded-b-none border-b-0",
              )}
            >
              <div className="flex items-center">
                <Button
                  size="xs"
                  variant="neutral"
                  className={cn(
                    "rounded-r-none border-0 border-border-transparent dark:border-border-transparent",
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
                    "rounded-none border-0 dark:border-border-transparent",
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
        {indexes && showFilters && (
          <div className="w-full animate-fadeInFromLoading">
            <div className="scrollbar flex w-full flex-col gap-2 overflow-x-auto rounded-sm rounded-tl-none border bg-background-secondary p-2 pb-2.5">
              <IndexFilters
                shownFilters={shownFilters}
                defaultDocument={defaultDocument}
                indexes={indexes}
                tableName={tableName}
                activeSchema={activeSchema}
                getValidatorForField={getValidatorForField}
                onFiltersChange={onFiltersChange}
                applyFiltersWithHistory={applyFiltersWithHistory}
                setDraftFilters={setDraftFilters}
                onChangeOrder={onChangeOrder}
                onChangeIndexFilter={onChangeIndexFilter}
                invalidFilters={invalidFilters}
                onError={(...args) => onError("index", ...args)}
                hasInvalidFilters={hasInvalidFilters}
              />
              {shownFilters.clauses.length > 0 && (
                <div className="mt-2 flex flex-col gap-2">
                  <div className="flex items-center gap-1">
                    <hr className="w-2" />{" "}
                    <p className="flex items-center gap-1 text-xs text-content-secondary">
                      Other Filters
                      <Tooltip
                        tip="Other filters are not indexed and are applied after the indexed filters. These filters are less efficient."
                        side="right"
                      >
                        <InfoCircledIcon />
                      </Tooltip>
                    </p>{" "}
                    <hr className="grow" />
                  </div>
                  {shownFilters.clauses.map((clause, idx) => (
                    <FilterItem
                      key={clause.id || idx}
                      idx={idx}
                      fields={tableFields}
                      defaultDocument={defaultDocument}
                      clause={clause}
                      onChangeFilter={onChangeFilter}
                      onDeleteFilter={onDeleteFilter}
                      onApplyFilters={() => {
                        if (hasInvalidFilters) {
                          return;
                        }
                        log("apply filters", {
                          hasIndexFilters:
                            (shownFilters.index?.clauses || []).filter(
                              (c) => c.enabled,
                            ).length > 0,
                          hasOtherFilters:
                            shownFilters.clauses.filter(
                              (c) => c.enabled !== false,
                            ).length > 0,
                        });
                        onFiltersChange(shownFilters);
                      }}
                      onError={(...args) => onError("filter", ...args)}
                      error={
                        clause.enabled !== false
                          ? dataFetchErrors?.find((e) => e.filter === idx)
                              ?.error || invalidFilters[`filter/${idx}`]
                          : undefined
                      }
                      autoFocusValueEditor={
                        idx === shownFilters.clauses.length - 1
                      }
                      documentValidator={getValidatorForField(clause.field)}
                      shouldSurfaceValidatorErrors={
                        activeSchema?.schemaValidation
                      }
                    />
                  ))}
                </div>
              )}
              <div className="mt-2 flex items-center gap-1">
                <Button
                  variant="neutral"
                  size="xs"
                  className="text-xs"
                  icon={<PlusIcon />}
                  onClick={() => {
                    onAddFilter(shownFilters.clauses.length);
                    log("add filter");
                  }}
                >
                  Add filter
                </Button>
                {isDirty || (dataFetchErrors && dataFetchErrors.length > 0) ? (
                  <Button
                    type="submit"
                    tip={
                      hasInvalidFilters
                        ? "Fix the errors above to apply your filters."
                        : undefined
                    }
                    disabled={hasInvalidFilters}
                    size="xs"
                    data-testid="apply-filters"
                    className="text-xs"
                  >
                    Apply Filters
                  </Button>
                ) : (
                  hasFilters && (
                    <div className="flex w-full items-center gap-1">
                      <p className="ml-1 flex gap-0.5 text-xs font-medium text-content-secondary">
                        <CheckIcon />
                        Filters applied
                      </p>
                      <Button
                        size="xs"
                        variant="neutral"
                        className="ml-auto text-xs"
                        onClick={() => {
                          onFiltersChange({
                            clauses: [],
                            index: shownFilters.index
                              ? {
                                  name: shownFilters.index.name,
                                  clauses: shownFilters.index.clauses.map(
                                    (clause) => ({
                                      ...clause,
                                      enabled: false,
                                    }),
                                  ) as
                                    | FilterByIndex[]
                                    | [...FilterByIndex[], FilterByIndexRange],
                                }
                              : undefined,
                          });
                        }}
                      >
                        Clear filters
                      </Button>
                    </div>
                  )
                )}
                {dataFetchErrors && dataFetchErrors.length > 0 && (
                  <p
                    className="h-4 text-xs break-words text-content-errorSecondary"
                    role="alert"
                  >
                    {dataFetchErrors[0].error}
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
          <div className="rounded-sm border bg-background-error p-1">
            <ExclamationTriangleIcon className="size-4 text-content-errorSecondary" />
          </div>
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
    enabled: true,
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
  onFiltersChange,
  draftFilters,
  setDraftFilters,
  activeSchema,
}: {
  tableName: string;
  componentId: string | null;
  filters?: FilterExpression;
  onFiltersChange(next: FilterExpression): void;
  draftFilters?: FilterExpression;
  setDraftFilters(next: FilterExpression): void;
  activeSchema: SchemaJson | null;
}) {
  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();
  const [invalidFilters, { set: setInvalidFilters }] = useMap();

  const isDirty = !isEqual(filters, draftFilters);
  const hasInvalidFilters =
    Object.entries(invalidFilters).filter(([k, v]) => {
      if (v === undefined) {
        return false;
      }

      const [namespace, idx] = k.split("/");
      const clauses =
        namespace === "filter"
          ? draftFilters?.clauses
          : draftFilters?.index?.clauses;
      return clauses?.[Number(idx)]?.enabled;
    }).length > 0;

  const shownFilters = useMemo(
    () =>
      draftFilters ??
      ({
        clauses: [],
        index: getDefaultIndex(),
      } as FilterExpression),
    [draftFilters],
  );

  const onChangeFilter = useCallback(
    (filter: FilterState, idx: number) => {
      const newFilters = cloneDeep(shownFilters);
      const oldFilter = newFilters.clauses[idx];

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
          enabled: filter.enabled,
        };
      }

      // Log filter changes
      if (oldFilter) {
        if (oldFilter.enabled !== filter.enabled) {
          log("filter toggle", {
            enabled: filter.enabled,
            filterType: "regular",
            filterIndex: idx,
          });
        } else if (oldFilter.op !== filter.op) {
          log("filter operator change", {
            oldOperator: oldFilter.op,
            newOperator: filter.op,
            filterType: "regular",
            filterIndex: idx,
          });
        } else if (oldFilter.field !== filter.field) {
          log("filter field change", {
            filterType: "regular",
            filterIndex: idx,
          });
        }
      }

      newFilters.clauses[idx] = newFilter;
      setDraftFilters(newFilters);
    },
    [shownFilters, setDraftFilters, log],
  );

  const onChangeIndexFilter = useCallback(
    (filter: IndexFilterState, idx: number) => {
      const newFilters = cloneDeep(shownFilters);
      if (!newFilters.index) {
        throw new Error("Index not found");
      }
      const oldFilter = newFilters.index.clauses[idx];

      // Log index filter changes
      if (oldFilter) {
        if (oldFilter.enabled !== filter.enabled) {
          log("filter toggle", {
            enabled: filter.enabled,
            filterType: "index",
            filterIndex: idx,
          });
        } else if (oldFilter.type !== filter.type) {
          log("index filter type change", {
            oldType: oldFilter.type,
            newType: filter.type,
            filterIndex: idx,
          });
        }
      }

      newFilters.index.clauses[idx] = filter;
      setDraftFilters(newFilters);
    },
    [shownFilters, setDraftFilters, log],
  );

  const onDeleteFilter = useCallback(
    (idx: number) => {
      log("filter delete", {
        filterType: "regular",
        filterIndex: idx,
      });
      setInvalidFilters(idx, undefined);
      const newFilters = {
        ...shownFilters,
        clauses: [
          ...shownFilters.clauses.slice(0, idx),
          ...shownFilters.clauses.slice(idx + 1),
        ],
        index: shownFilters.index || getDefaultIndex(),
      } as FilterExpression;
      setDraftFilters(newFilters);
    },
    [shownFilters, setDraftFilters, setInvalidFilters, log],
  );

  const onAddFilter = useCallback(
    (idx: number) => {
      log("filter add", {
        filterType: "regular",
        filterIndex: idx,
      });
      const newFilters = {
        ...shownFilters,
        clauses: [
          ...shownFilters.clauses.slice(0, idx),
          generateNewFilter(),
          ...shownFilters.clauses.slice(idx),
        ],
        index: shownFilters.index || getDefaultIndex(),
      } as FilterExpression;
      setDraftFilters(newFilters);
    },
    [shownFilters, setDraftFilters, log],
  );

  const onError = useCallback(
    (namespace: "filter" | "index", idx: number, errors: string[]) => {
      setInvalidFilters(
        `${namespace}/${idx}`,
        errors.length ? errors[0] : undefined,
      );
    },
    [setInvalidFilters],
  );

  const { filterHistory } = useFilterHistory(tableName, componentId);
  const { applyFiltersWithHistory } = useTableFilters(tableName, componentId);
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

  const onChangeOrder = useCallback(
    (newOrder: "asc" | "desc") => {
      log("filter order change", {
        oldOrder: shownFilters.order,
        newOrder,
      });
      const newFilters = {
        ...shownFilters,
        clauses: shownFilters.clauses.map((filter, idx) => ({
          ...filter,
          enabled: invalidFilters[`filter/${idx}`] ? false : filter.enabled,
        })),
        order: newOrder,
      };
      setDraftFilters(newFilters);
      onFiltersChange(newFilters);
    },
    [shownFilters, setDraftFilters, onFiltersChange, invalidFilters, log],
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
    onChangeOrder,
    getValidatorForField,
    onChangeIndexFilter,
    applyFiltersWithHistory,
  };
}
