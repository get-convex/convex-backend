import { BackspaceIcon } from "@heroicons/react/24/outline";
import {
  BarChartIcon,
  TrashIcon,
  DotsVerticalIcon,
  CodeIcon,
  CardStackIcon,
} from "@radix-ui/react-icons";
import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useNents } from "@common/lib/useNents";
import { Menu, MenuItem } from "@common/elements/Menu";
import { TableSchemaStatus } from "@common/features/data/components/TableSchema";

export function DataOverflowMenu({
  tableSchemaStatus,
  numRows,
  onClickCustomQuery,
  onClickClearTable,
  onClickSchemaIndexes,
  onClickMetrics,
  onClickDeleteTable,
}: {
  tableSchemaStatus: TableSchemaStatus | undefined;
  numRows: number;
  onClickCustomQuery: () => void;
  onClickClearTable: () => void;
  onClickSchemaIndexes: () => void;
  onClickMetrics: () => void;
  onClickDeleteTable: () => void;
}) {
  const { selectedNent } = useNents();

  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );
  const isInSchema =
    tableSchemaStatus?.isDefined ||
    tableSchemaStatus?.referencedByTable !== undefined;

  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );

  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canManageTable =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;
  return (
    <Menu
      placement="bottom-start"
      buttonProps={{
        "aria-label": "Open table settings",
        icon: <DotsVerticalIcon className="m-[3px]" />,
        size: "sm",
        variant: "neutral",
      }}
    >
      <MenuItem action={onClickCustomQuery}>
        <CodeIcon />
        Custom query
      </MenuItem>
      <MenuItem action={onClickSchemaIndexes}>
        <CardStackIcon />
        Schema and Indexes
      </MenuItem>
      <MenuItem action={onClickMetrics}>
        <BarChartIcon />
        Metrics
      </MenuItem>
      <MenuItem
        tip={
          isInUnmountedComponent
            ? "Cannot clear tables in an unmounted component."
            : numRows === 0
              ? "There are no documents to delete."
              : !canManageTable
                ? "You do not have permission to clear tables in production."
                : undefined
        }
        tipSide="left"
        variant="danger"
        action={onClickClearTable}
        disabled={numRows === 0 || !canManageTable || isInUnmountedComponent}
      >
        <BackspaceIcon className="h-4 w-4" />
        Clear Table
      </MenuItem>
      <MenuItem
        tip={
          isInUnmountedComponent ? (
            "Cannot delete tables in an unmounted component."
          ) : isInSchema ? (
            <RemoveTableFromSchemaTip tableSchemaStatus={tableSchemaStatus} />
          ) : !canManageTable ? (
            "You do not have permission to delete tables in production."
          ) : undefined
        }
        tipSide="left"
        variant="danger"
        action={onClickDeleteTable}
        disabled={
          isInSchema ||
          !canManageTable ||
          isInUnmountedComponent ||
          tableSchemaStatus?.isValidationRunning
        }
      >
        <TrashIcon />
        Delete Table
      </MenuItem>
    </Menu>
  );
}

function RemoveTableFromSchemaTip({
  tableSchemaStatus,
}: {
  tableSchemaStatus: TableSchemaStatus | undefined;
}) {
  if (tableSchemaStatus === undefined) {
    // In case we can't tell whether the table is in the schema, show a generic tip.
    return (
      <p>You cannot delete this table because it is defined in your schema.</p>
    );
  }
  if (tableSchemaStatus.isDefined) {
    return (
      <>
        <p>
          You cannot delete the table "{tableSchemaStatus.tableName}" because it
          is defined in your schema.
        </p>
        <p>
          Before you can delete it, you need to remove the table "
          {tableSchemaStatus.tableName}" including occurrences of{" "}
          <code>v.id("{tableSchemaStatus.tableName}")</code> from your
          "schema.ts" file.
        </p>
      </>
    );
  }
  if (tableSchemaStatus.referencedByTable) {
    return (
      <p>
        You cannot delete the table "{tableSchemaStatus.tableName}" because it
        is referenced as <code>v.id("{tableSchemaStatus.tableName}")</code> in
        the schema for table "{tableSchemaStatus.referencedByTable}".
      </p>
    );
  }
  return null;
}
