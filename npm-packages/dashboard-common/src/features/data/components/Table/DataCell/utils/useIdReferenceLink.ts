import { useQuery } from "convex/react";
import { Value } from "convex/values";
import udfs from "@common/udfs";
import { stringifyValue } from "@common/lib/stringifyValue";
import { useNents } from "@common/lib/useNents";
import { documentHref, getReferencedTableName } from "@common/lib/utils";
import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function useIdReferenceLink(value: Value, columnName: string) {
  const stringValue = typeof value === "string" ? value : stringifyValue(value);

  const componentId = useNents().selectedNent?.id ?? null;
  const tableMapping = useQuery(udfs.getTableMapping.default, {
    componentId,
  });
  const referencedTableName = getReferencedTableName(tableMapping, value);
  const isReference = referencedTableName !== null;

  const { deploymentsURI } = useContext(DeploymentInfoContext);

  if (columnName === "_id") {
    return undefined;
  }

  const link =
    isReference && referencedTableName
      ? documentHref({
          deploymentsURI,
          tableName: referencedTableName,
          id: stringValue,
          componentId,
        })
      : undefined;

  return link;
}
