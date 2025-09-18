import React, { useContext } from "react";
import { ArrowsUpDownIcon, FingerPrintIcon } from "@heroicons/react/24/outline";
import {
  ClockIcon,
  IdCardIcon,
  InfoCircledIcon,
  MagnifyingGlassIcon,
} from "@radix-ui/react-icons";
import { GenericDocument } from "convex/server";
import { convexToJson, ValidatorJSON } from "convex/values";
import {
  DatabaseIndexFilter,
  DatabaseIndexFilterClause,
  FilterByIndexRange,
  FilterExpression,
  SearchIndexFilter,
  SearchIndexFilterClause,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { Tooltip } from "@ui/Tooltip";
import { SchemaJson } from "@common/lib/format";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Index } from "@common/features/data/lib/api";
import { cn } from "@ui/cn";
import { DatabaseIndexFilterEditor } from "./DatabaseIndexFilterEditor";
import { SearchValueEditor } from "./SearchValueEditor";
import { SearchIndexFilterEditor } from "./SearchIndexFilterEditor";

export function getDefaultIndex(): {
  name: string;
  clauses: [FilterByIndexRange];
} {
  return {
    name: DEFAULT_INDEX_NAME,
    clauses: [getDefaultIndexClause()],
  };
}

// Function to generate a default index clause with current timestamp
function getDefaultIndexClause(): FilterByIndexRange {
  return {
    type: "indexRange",
    enabled: false,
    lowerOp: "gte",
    lowerValue: new Date().getTime(),
  };
}

// Define a simplified Index type for our use
type SimpleIndex = {
  name: string;
  fields: string[];
};

// Define a type for our index option value
type IndexOptionValue =
  | {
      name: string;
      fields: string[];
      type: "default" | "database";
    }
  | {
      name: string;
      searchField: string;
      fields: string[];
      type: "search";
    };

// Create constants for the default index
const DEFAULT_INDEX_NAME = "by_creation_time";
const DEFAULT_INDEX_FIELD = "_creationTime";
const DEFAULT_INDEX_LABEL = "By creation time";

const BY_ID_INDEX_NAME = "by_id";
const BY_ID_INDEX_FIELD = "_id";
const BY_ID_INDEX_LABEL = "By ID";

// Define the default index object to reuse throughout the component
const DEFAULT_INDEX: IndexOptionValue = {
  name: DEFAULT_INDEX_NAME,
  fields: [DEFAULT_INDEX_FIELD],
  type: "default",
};

const BY_ID_INDEX: IndexOptionValue = {
  name: BY_ID_INDEX_NAME,
  fields: [BY_ID_INDEX_FIELD],
  type: "default",
};

type IndexFiltersProps = {
  shownFilters: FilterExpression;
  defaultDocument: GenericDocument;
  indexes: Index[] | undefined;
  tableName: string;
  activeSchema: SchemaJson | null;
  getValidatorForField: (fieldName?: string) => ValidatorJSON | undefined;
  onFiltersChange: (next: FilterExpression) => void;
  applyFiltersWithHistory: (next: FilterExpression) => Promise<void>;
  setDraftFilters: (next: FilterExpression) => void;
  onChangeOrder: (newOrder: "asc" | "desc") => void;
  onChangeIndexFilter: (
    filter: DatabaseIndexFilterClause | SearchIndexFilterClause,
    idx: number,
  ) => void;
  onError: (idx: number, errors: string[]) => void;
  hasInvalidFilters: boolean;
  invalidFilters: Record<string, string>;
};

export function IndexFilters({
  shownFilters,
  defaultDocument,
  invalidFilters,
  indexes,
  tableName,
  activeSchema,
  getValidatorForField,
  onFiltersChange,
  applyFiltersWithHistory,
  setDraftFilters,
  onChangeOrder,
  onChangeIndexFilter,
  onError,
  hasInvalidFilters,
}: IndexFiltersProps) {
  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();

  const indexOptions: { value: IndexOptionValue; label: string }[] = indexes
    ? [
        // Add the by_id system index
        {
          value: DEFAULT_INDEX,
          label: DEFAULT_INDEX_LABEL,
        },
        // Add the default by_creation_time index
        {
          value: BY_ID_INDEX,
          label: BY_ID_INDEX_LABEL,
        },
        // Add user database and search indexes
        ...indexes
          .filter(
            (index) =>
              !index.staged &&
              (Array.isArray(index.fields) || "searchField" in index.fields),
          )
          .map((index) => {
            if ("vectorField" in index.fields) {
              // Unreachable: vector indexes are filtered out above
              throw new Error("Unexpected vector index");
            }

            const value: IndexOptionValue =
              "searchField" in index.fields
                ? {
                    name: index.name,
                    searchField: index.fields.searchField,
                    fields: index.fields.filterFields,
                    type: "search",
                  }
                : {
                    name: index.name,
                    fields: index.fields,
                    type: "database",
                  };

            return {
              value,
              label: index.name,
            };
          }),
      ]
    : [];

  const selectedTableIndex = indexes?.find(
    (index) => index.name === shownFilters.index?.name,
  );
  const searchIndex =
    shownFilters.index && "search" in shownFilters.index
      ? shownFilters.index
      : null;
  const searchFilterField =
    searchIndex &&
    selectedTableIndex &&
    "searchField" in selectedTableIndex.fields
      ? selectedTableIndex.fields.searchField
      : null;

  return (
    <>
      <div className="mb-1 flex items-center gap-1.5">
        <Combobox
          size="sm"
          label="Sort by Index"
          options={indexOptions}
          buttonClasses="w-fit"
          buttonProps={{
            tip: "Use an index to sort your data and improve filter performance.",
            tipSide: "right",
          }}
          optionsWidth="fit"
          innerButtonClasses="text-xs w-fit pl-1"
          selectedOption={
            indexOptions.find((o) => o.value.name === shownFilters.index?.name)
              ?.value ?? DEFAULT_INDEX
          }
          setSelectedOption={(option: IndexOptionValue | null) => {
            if (!option) {
              return;
            }

            log("sort by index combobox opened", {
              selectedOption: option.name,
            });

            // Clear all errors for the existing index filters
            shownFilters.index?.clauses.forEach((_, idx) => {
              onError(idx, []);
            });

            const newFilters: FilterExpression =
              option.type === "search"
                ? {
                    // TODO(ENG-9733) Support arbitrary filters in search queries
                    clauses: [],
                    order: "asc",
                    index: {
                      name: option.name,
                      search: searchIndex ? searchIndex.search : "",
                      clauses: option.fields.map((field: string) => ({
                        field,
                        enabled: false,
                        value:
                          defaultDocument[field] === undefined
                            ? undefined
                            : convexToJson(defaultDocument[field]),
                      })),
                    } satisfies SearchIndexFilter,
                  }
                : {
                    clauses: shownFilters.clauses,
                    order:
                      // When going from a search index to a database index,
                      // reset `order` to the default value
                      searchIndex ? undefined : shownFilters.order,
                    index: {
                      name: option.name,
                      clauses: option.fields.map((field: string) => ({
                        type: "indexEq",
                        enabled: false,
                        value:
                          field === "_creationTime"
                            ? new Date().getTime()
                            : field === "_id"
                              ? ""
                              : defaultDocument[field],
                      })),
                    } satisfies DatabaseIndexFilter,
                  };
            setDraftFilters(newFilters);
            onFiltersChange(newFilters);
          }}
          Option={IndexOption}
        />
        {!searchIndex && (
          // Search indexes only support a single order, so we don’t display the order button for them
          <Button
            variant="neutral"
            size="xs"
            onClick={() =>
              onChangeOrder(shownFilters.order === "asc" ? "desc" : "asc")
            }
            type="button"
            tip="Change sort order"
            className="w-fit text-xs"
            icon={<ArrowsUpDownIcon className="size-4" />}
          >
            {shownFilters.order === "asc" ? "Ascending" : "Descending"}
          </Button>
        )}
      </div>
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-1">
          <hr className="w-2" />{" "}
          <p className="flex items-center gap-1 text-xs text-content-secondary">
            Indexed Filters{" "}
            <Tooltip
              tip="Indexed filters are automatically generated based on the index selected above."
              side="right"
            >
              <InfoCircledIcon />
            </Tooltip>
          </p>{" "}
          <hr className="grow" />
        </div>
        {/* Regular index filters */}
        {shownFilters.index &&
          !searchIndex &&
          shownFilters.index.clauses.map((clause, idx) => {
            if ("field" in clause) {
              throw new Error(
                "Unexpected search index clause in a regular index filter",
              );
            }

            // Get the index definition
            const tableIndexes =
              activeSchema?.tables.find(
                (t: { tableName: string }) => t.tableName === tableName,
              )?.indexes || [];

            const indexName = shownFilters.index?.name;

            // Get the field name from the index definition
            let fieldName =
              selectedTableIndex && Array.isArray(selectedTableIndex.fields)
                ? selectedTableIndex.fields[idx]
                : "_creationTime";

            // Special case for system indexes
            if (indexName === "by_creation_time") {
              fieldName = "_creationTime";
            } else if (indexName === "by_id") {
              fieldName = "_id";
            } else {
              const indexDef = tableIndexes.find((i: any) => {
                const simpleIndex = i as unknown as SimpleIndex;
                return simpleIndex.name === indexName;
              });

              if (indexDef) {
                const simpleIndex = indexDef as unknown as SimpleIndex;
                if (Array.isArray(simpleIndex.fields)) {
                  fieldName = simpleIndex.fields[idx] || fieldName;
                }
              }
            }

            // Calculate if all previous filters are enabled
            const previousFiltersEnabled =
              shownFilters.index?.clauses.slice(0, idx).map((c) => c.enabled) ||
              [];

            // Calculate if any subsequent filters are enabled
            const nextFiltersEnabled =
              shownFilters.index?.clauses
                .slice(idx + 1)
                .map((c) => c.enabled) || [];

            return (
              <DatabaseIndexFilterEditor
                key={idx}
                idx={idx}
                field={fieldName}
                error={
                  clause.enabled ? invalidFilters[`index/${idx}`] : undefined
                }
                onChange={onChangeIndexFilter}
                onApplyFilters={async () => {
                  if (hasInvalidFilters) {
                    return;
                  }
                  await applyFiltersWithHistory(shownFilters);
                }}
                onError={onError}
                filter={clause}
                autoFocusValueEditor={
                  idx === (shownFilters.index?.clauses.length || 0) - 1
                }
                documentValidator={getValidatorForField(fieldName)}
                shouldSurfaceValidatorErrors={activeSchema?.schemaValidation}
                previousFiltersEnabled={previousFiltersEnabled}
                nextFiltersEnabled={nextFiltersEnabled}
              />
            );
          })}

        {searchIndex && (
          <>
            <SearchValueEditor
              field={searchFilterField ?? "unknown"}
              value={searchIndex.search}
              onChange={(newValue: string) => {
                const newFilters: FilterExpression = {
                  ...shownFilters,
                  index: {
                    ...searchIndex,
                    search: newValue,
                  },
                };
                setDraftFilters(newFilters);
              }}
              onApplyFilters={async () => {
                if (hasInvalidFilters) {
                  return;
                }
                await applyFiltersWithHistory(shownFilters);
              }}
              indented={searchIndex.clauses.length > 0}
            />

            {searchIndex.clauses.map((clause, idx) => (
              <SearchIndexFilterEditor
                key={clause.field}
                idx={idx}
                field={clause.field}
                error={
                  clause.enabled ? invalidFilters[`index/${idx}`] : undefined
                }
                onChange={onChangeIndexFilter}
                onApplyFilters={async () => {
                  if (hasInvalidFilters) {
                    return;
                  }
                  await applyFiltersWithHistory(shownFilters);
                }}
                onError={onError}
                filter={clause}
                autoFocusValueEditor={
                  idx === (shownFilters.index?.clauses.length || 0) - 1
                }
                documentValidator={getValidatorForField(clause.field)}
                shouldSurfaceValidatorErrors={activeSchema?.schemaValidation}
              />
            ))}
          </>
        )}
      </div>
    </>
  );
}

export function IndexOption({
  label,
  value,
  inButton,
}: {
  label: string;
  value: IndexOptionValue;
  inButton: boolean;
}) {
  return (
    <div className="flex items-center gap-2 text-xs">
      <div className="text-content-tertiary">
        {inButton ? (
          <FingerPrintIcon className="size-4 text-content-primary" />
        ) : value.type === "database" ? (
          <Tooltip side="left" tip="Index">
            <FingerPrintIcon className="size-4" />
          </Tooltip>
        ) : value.type === "search" ? (
          <Tooltip
            side="left"
            tip="Search index"
            className="inline-flex size-4 justify-center"
          >
            <MagnifyingGlassIcon />
          </Tooltip>
        ) : value.name === DEFAULT_INDEX_NAME ? (
          <ClockIcon />
        ) : (
          <IdCardIcon />
        )}
      </div>

      <div>
        <div>
          {value.type !== "default" && inButton && (
            <>
              <span>{value.type === "search" ? "Search index" : "Index"}:</span>{" "}
            </>
          )}
          <span className={cn(value.type !== "default" && "font-mono")}>
            {label}
          </span>
        </div>

        {!inButton && (
          <div className="text-xs text-content-secondary">
            (
            {("searchField" in value ? [value.searchField] : value.fields).join(
              ", ",
            )}
            )
          </div>
        )}
      </div>
    </div>
  );
}
