import React, { useContext } from "react";
import { ArrowsUpDownIcon, FingerPrintIcon } from "@heroicons/react/24/outline";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { GenericDocument } from "convex/server";
import { ValidatorJSON } from "convex/values";
import {
  FilterByIndexRange,
  DatabaseFilterExpression,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { Tooltip } from "@ui/Tooltip";
import { SchemaJson } from "@common/lib/format";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Index } from "@common/features/data/lib/api";
import { IndexFilterEditor, IndexFilterState } from "./IndexFilterEditor";

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
type IndexOptionValue = {
  name: string;
  fields: string[];
};

// Create constants for the default index
const DEFAULT_INDEX_NAME = "by_creation_time";
const DEFAULT_INDEX_FIELD = "_creationTime";
const DEFAULT_INDEX_LABEL = DEFAULT_INDEX_FIELD;

const BY_ID_INDEX_NAME = "by_id";
const BY_ID_INDEX_FIELD = "_id";
const BY_ID_INDEX_LABEL = BY_ID_INDEX_FIELD;

// Define the default index object to reuse throughout the component
const DEFAULT_INDEX = {
  name: DEFAULT_INDEX_NAME,
  fields: [DEFAULT_INDEX_FIELD],
};

const BY_ID_INDEX = {
  name: BY_ID_INDEX_NAME,
  fields: [BY_ID_INDEX_FIELD],
};

type IndexFiltersProps = {
  shownFilters: DatabaseFilterExpression;
  defaultDocument: GenericDocument;
  indexes: Index[] | undefined;
  tableName: string;
  activeSchema: SchemaJson | null;
  getValidatorForField: (fieldName?: string) => ValidatorJSON | undefined;
  onFiltersChange: (next: DatabaseFilterExpression) => void;
  applyFiltersWithHistory: (next: DatabaseFilterExpression) => Promise<void>;
  setDraftFilters: (next: DatabaseFilterExpression) => void;
  onChangeOrder: (newOrder: "asc" | "desc") => void;
  onChangeIndexFilter: (filter: IndexFilterState, idx: number) => void;
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

  // Restructure indexOptions to use the Option<IndexOptionValue> type
  const indexOptions = indexes
    ? [
        // Add the by_id system index
        // Add the default by_creation_time index
        {
          value: {
            name: DEFAULT_INDEX.name,
            fields: DEFAULT_INDEX.fields,
          },
          label: DEFAULT_INDEX_LABEL,
        },
        {
          value: {
            name: BY_ID_INDEX.name,
            fields: BY_ID_INDEX.fields,
          },
          label: BY_ID_INDEX_LABEL,
        },
        // Add other user indexes with array fields
        ...indexes
          .filter((index) => Array.isArray(index.fields) && !index.staged)
          .map((index) => {
            const simpleIndex = index as unknown as SimpleIndex;
            return {
              value: {
                name: simpleIndex.name,
                fields: simpleIndex.fields,
              },
              label: simpleIndex.name,
            };
          }),
      ]
    : [];

  const selectedTableIndex = indexes?.find(
    (index) => index.name === shownFilters.index?.name,
  );

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
          innerButtonClasses="text-xs w-fit pl-1"
          selectedOption={
            indexOptions.find((o) => o.value.name === shownFilters.index?.name)
              ?.value || {
              name: DEFAULT_INDEX.name,
              fields: DEFAULT_INDEX.fields,
            }
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

            const newFilters = {
              ...shownFilters,
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
              },
            } as DatabaseFilterExpression;
            setDraftFilters(newFilters);
            onFiltersChange(newFilters);
          }}
          Option={({ inButton, label, value }) => (
            <div className="flex items-center gap-1 text-xs">
              {inButton && (
                <>
                  <FingerPrintIcon className="size-4" />
                  Index:
                </>
              )}
              <span>{label}</span>
              {!inButton && (
                <span className="text-xs text-content-secondary">
                  ({value.fields.join(", ")})
                </span>
              )}
            </div>
          )}
        />
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
        {shownFilters.index?.clauses.map((clause, idx) => {
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
            shownFilters.index?.clauses.slice(idx + 1).map((c) => c.enabled) ||
            [];

          return (
            <IndexFilterEditor
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
      </div>
    </>
  );
}
