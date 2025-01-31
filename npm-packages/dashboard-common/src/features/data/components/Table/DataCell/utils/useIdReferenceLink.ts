import { useQuery } from "convex/react";
import { Value } from "convex/values";
import { useRouter } from "next/router";
import udfs from "udfs";
import { stringifyValue } from "lib/stringifyValue";
import { useNents } from "lib/useNents";
import { documentHref, getReferencedTableName } from "lib/utils";

export function useIdReferenceLink(value: Value, columnName: string) {
  const stringValue = typeof value === "string" ? value : stringifyValue(value);

  const tableMapping = useQuery(udfs.getTableMapping.default, {
    componentId: useNents().selectedNent?.id ?? null,
  });
  const referencedTableName = getReferencedTableName(tableMapping, value);
  const isReference = referencedTableName !== null;

  const router = useRouter();

  if (columnName === "_id") {
    return undefined;
  }

  const link =
    isReference && referencedTableName
      ? documentHref(router, referencedTableName, stringValue)
      : undefined;

  return link;
}
