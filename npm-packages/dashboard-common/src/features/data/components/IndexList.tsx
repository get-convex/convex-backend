import React from "react";
import { Index } from "@common/features/data/lib/api";
import { Tooltip } from "@ui/Tooltip";
import { ProgressBar } from "@ui/ProgressBar";
import { useQuery } from "convex/react";
import { api } from "system-udfs/convex/_generated/api";
import { useNents } from "@common/lib/useNents";

function IndexRow({ index }: { index: Index }) {
  const { type, fields } = getIndexDescription(index);
  return (
    <tr className="border-b">
      <td className="max-w-[6rem] divide-y divide-border-transparent truncate py-2 pr-8 font-mono text-sm text-content-secondary sm:max-w-[16rem]">
        {index.name}
      </td>
      <td className="max-w-[9rem] truncate py-2 pr-8 font-mono text-sm text-content-secondary sm:max-w-[24rem]">
        {fields}
      </td>
      <td className="ml-auto w-full py-2 text-sm text-content-secondary">
        {index.backfill.state !== "done" ? (
          <div className="flex items-center gap-2">
            <Tooltip tip="This index is currently backfilling, and is not yet ready to use.">
              {index.backfill.stats?.totalDocs !== null &&
                index.backfill.stats?.totalDocs !== undefined &&
                index.backfill.stats?.totalDocs > 0 &&
                index.backfill.stats?.numDocsIndexed !== undefined && (
                  <IndexBackfillProgress
                    numDocsIndexed={index.backfill.stats.numDocsIndexed}
                    totalDocs={index.backfill.stats.totalDocs}
                  />
                )}
            </Tooltip>
          </div>
        ) : (
          type
        )}
      </td>
    </tr>
  );
}

function IndexBackfillProgress({
  numDocsIndexed,
  totalDocs,
}: {
  numDocsIndexed: number;
  totalDocs: number;
}) {
  const fraction = Math.min(numDocsIndexed / totalDocs, 0.99);
  const percent = Math.round(fraction * 100);
  return (
    <div className="flex items-center gap-2">
      <ProgressBar
        fraction={fraction}
        ariaLabel={`Index backfill progress: ${percent}%  of documents`}
        variant="stripes"
        className="w-24"
      />
      <span className="text-xs text-content-tertiary">{percent}%</span>
    </div>
  );
}

function getIndexDescription(index: Index) {
  if (index.fields instanceof Array) {
    return { type: "", fields: index.fields.join(", ") };
  }
  if ("searchField" in index.fields) {
    return { type: "text search index", fields: index.fields.searchField };
  }
  return { type: "vector search index", fields: index.fields.vectorField };
}

export function IndexesList({ indexes }: { indexes?: Index[] }) {
  if (!indexes || indexes.length === 0) {
    return <>This table has no indexes</>;
  }

  return (
    <table className="table-auto">
      <thead>
        <tr className="border-b">
          <th className="max-w-[6rem] divide-y py-2 pr-8 text-left text-sm font-semibold text-content-secondary sm:max-w-[16rem]">
            Name
          </th>
          <th className="max-w-[9rem] py-2 pr-8 text-left text-sm font-semibold text-content-secondary sm:max-w-[24rem]">
            Fields
          </th>
          <th aria-label="type and details" />
        </tr>
      </thead>

      <tbody>
        {indexes.map((index, i) => (
          <IndexRow key={`${index.name}-${i}`} index={index} />
        ))}
      </tbody>
    </table>
  );
}

export function IndexList({ tableName }: { tableName: string }) {
  const { selectedNent } = useNents();
  const indexes =
    useQuery(api._system.frontend.indexes.default, {
      tableName,
      tableNamespace: selectedNent?.id ?? null,
    }) ?? undefined;

  return <IndexesList indexes={indexes} />;
}
