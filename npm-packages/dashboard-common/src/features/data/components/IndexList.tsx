import { Callout, Spinner, Tooltip } from "dashboard-common";
import React from "react";
import { Index, useTableIndexes } from "../lib/api";

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
          <Tooltip tip="This index is currently backfilling, and is not yet ready to use.">
            <span className="h-5 w-5">
              <Spinner />
            </span>
          </Tooltip>
        ) : (
          type
        )}
      </td>
    </tr>
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
  return !indexes || indexes.length === 0 ? (
    <>This table has no indexes</>
  ) : (
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
        {indexes.map((index) => (
          <IndexRow key={index.name} index={index} />
        ))}
      </tbody>
    </table>
  );
}

export function IndexList({ tableName }: { tableName: string }) {
  const { indexes, hadError } = useTableIndexes(tableName);

  return hadError ? (
    <Callout variant="error">Encountered an error loading indexes.</Callout>
  ) : (
    <IndexesList indexes={indexes} />
  );
}
