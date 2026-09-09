import { ArrowDownIcon, ArrowRightIcon } from "@radix-ui/react-icons";
import { ReactNode, useMemo } from "react";
import { JSONValue } from "convex/values";
import {
  DatabaseIndexFilter,
  FilterExpression,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { cn } from "@ui/cn";
import { Link } from "@ui/Link";
import { Modal } from "@ui/Modal";
import { Index } from "@common/features/data/lib/api";
import { IndexFilters } from "@common/features/data/components/DataFilters/IndexFilters";
import { IndexFilterBar } from "./IndexFilterBar";
import { buildIndexDefs } from "./filterModel";
import { useFilterActions } from "./useFilterActions";

const noop = () => {};
const asyncNoop = async () => {};

const EXAMPLE_TIME = new Date("2026-01-01T00:00:00Z").getTime();

export function WhyChangedDialog({
  onClose,
  onSendFeedback,
  tableName,
  indexName = "by_author_channel",
  fields = ["author", "channel", "_creationTime"],
  values,
}: {
  onClose: () => void;
  onSendFeedback?: () => void;
  tableName: string;
  indexName?: string;
  fields?: string[];
  values: JSONValue[];
}) {
  const indexes: Index[] = useMemo(
    () => [{ name: indexName, fields, backfill: { state: "done" } }],
    [indexName, fields],
  );
  const indexDefs = useMemo(() => buildIndexDefs(indexes), [indexes]);

  const legacyFilters: FilterExpression = useMemo(
    () => ({
      clauses: [],
      order: "desc",
      index: {
        name: indexName,
        clauses: fields.map((field, i) =>
          i < values.length
            ? { type: "indexEq" as const, enabled: true, value: values[i] }
            : field === "_creationTime"
              ? {
                  type: "indexRange" as const,
                  enabled: false,
                  lowerOp: "gte" as const,
                  lowerValue: EXAMPLE_TIME,
                }
              : { type: "indexEq" as const, enabled: false, value: undefined },
        ) as DatabaseIndexFilter["clauses"],
      },
    }),
    [indexName, fields, values],
  );
  const indexFilters: FilterExpression = useMemo(
    () => ({
      clauses: [],
      order: "desc",
      index: {
        name: indexName,
        clauses: values.map((value) => ({
          type: "indexEq" as const,
          enabled: true,
          value,
        })),
      },
    }),
    [indexName, values],
  );

  const actions = useFilterActions({
    filters: indexFilters,
    draftFilters: indexFilters,
    setDraftFilters: noop,
    applyFilters: noop,
    indexDefs,
    defaultDocument: {},
  });

  return (
    <Modal title="Why did the filter UI change?" onClose={onClose} size="lg">
      <div className="flex flex-col gap-6">
        <div className="flex flex-col gap-4">
          <p className="max-w-prose text-sm text-content-primary">
            We're updating the data filtering experience to be simpler. Less
            clicks, less visual clutter, and a clearer view of which filters are
            currently applied.
          </p>
          <p className="max-w-prose text-sm text-content-primary">
            Both versions prioritize explicitly filtering using an index, which
            is the correct way to{" "}
            <Link
              href="https://stack.convex.dev/queries-that-scale"
              target="_blank"
            >
              make your queries perform well
            </Link>
            .
          </p>
        </div>

        <div className="flex flex-col gap-2 md:flex-row md:items-start md:gap-3">
          <Example label="Before" legacy>
            <IndexFilters
              shownFilters={legacyFilters}
              defaultDocument={{}}
              indexes={indexes}
              tableName={tableName}
              activeSchema={null}
              getValidatorForField={() => undefined}
              onFiltersChange={noop}
              applyFiltersWithHistory={asyncNoop}
              setDraftFilters={noop}
              onChangeOrder={noop}
              onChangeIndexFilter={noop}
              onError={noop}
              hasInvalidFilters={false}
              invalidFilters={{}}
            />
          </Example>
          <ArrowDownIcon
            className="size-5 shrink-0 self-center text-content-tertiary md:hidden"
            aria-hidden
          />
          <ArrowRightIcon
            className="hidden size-5 shrink-0 self-center text-content-tertiary md:block"
            aria-hidden
          />
          <Example label="After">
            <IndexFilterBar
              example
              actions={actions}
              indexDefs={indexDefs}
              tableName={tableName}
              tableFields={fields}
              defaultDocument={{}}
              activeSchema={null}
              numRowsLoaded={0}
              hasFilters={false}
              allFields={fields}
              hiddenColumns={[]}
              setHiddenColumns={noop}
              columnOrder={[]}
              setColumnOrder={noop}
            />
          </Example>
        </div>

        {onSendFeedback && (
          <p className="text-sm text-content-secondary">
            Have a different opinion?{" "}
            <Link
              href="#"
              onClick={(e) => {
                e.preventDefault();
                onClose();
                onSendFeedback();
              }}
            >
              Let us know
            </Link>
            .
          </p>
        )}
      </div>
    </Modal>
  );
}

function Example({
  label,
  legacy = false,
  children,
}: {
  label: string;
  legacy?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1">
      <span className="text-xs text-content-tertiary">{label}</span>
      <div
        inert
        className={cn(
          "scrollbar w-full overflow-x-auto rounded-lg border p-2",
          legacy && "border-dashed",
        )}
      >
        {children}
      </div>
    </div>
  );
}
