import { GenericDocument } from "convex/server";
import { useState } from "react";
import { TextInput } from "@ui/TextInput";
import { ProductionEditsConfirmationDialog } from "@common/elements/ProductionEditsConfirmationDialog";
import { EditDocumentField } from "@common/features/data/components/Table/EditDocumentField";
import {
  documentValidatorForTable,
  validatorForColumn,
} from "@common/features/data/components/Table/utils/validators";
import { SchemaJson, displayObjectFieldSchema } from "@common/lib/format";
import { useNents } from "@common/lib/useNents";
import { CopyButton } from "@common/elements/CopyButton";
import { stringifyValue } from "@common/lib/stringifyValue";
import { Button } from "@ui/Button";
import { ValidatorTooltip } from "./ValidatorTooltip";

export function ViewDocument({
  rows,
  columns,
  tableName,
  componentId,
  canManageTable,
  areEditsAuthorized,
  onAuthorizeEdits,
  activeSchema,
}: {
  rows: GenericDocument[];
  columns: string[];
  tableName: string;
  componentId: string | null;
  canManageTable: boolean;
  areEditsAuthorized: boolean;
  onAuthorizeEdits?: () => void;
  activeSchema: SchemaJson | null;
}) {
  const [showEnableProdEditsModalForColumn, setShowEnableProdEditsModal] =
    useState<string | undefined>(undefined);
  const [query, setQuery] = useState("");
  const [editingColumn, setEditingColumn] = useState<string | undefined>(
    undefined,
  );

  const { selectedNent } = useNents();
  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );

  let allowTopLevelUndefined = true;

  const documentValidator =
    activeSchema && documentValidatorForTable(activeSchema, tableName);

  const validator =
    editingColumn && documentValidator
      ? validatorForColumn(documentValidator, editingColumn)
      : undefined;

  // If we're doing validation, and the column is not optional, we don't want to allow top-level undefined.
  if (
    validator &&
    editingColumn &&
    documentValidator?.type === "object" &&
    !documentValidator.value[editingColumn]?.optional
  ) {
    allowTopLevelUndefined = false;
  }

  const shouldSurfaceValidatorErrors = activeSchema?.schemaValidation;

  return (
    <div className="flex h-full w-full min-w-[10rem] flex-col items-start overflow-y-hidden rounded-r border-l bg-background-secondary/70">
      {showEnableProdEditsModalForColumn && (
        <ProductionEditsConfirmationDialog
          onClose={() => {
            setShowEnableProdEditsModal(undefined);
          }}
          onConfirm={async () => {
            onAuthorizeEdits && onAuthorizeEdits();
            setEditingColumn(showEnableProdEditsModalForColumn);
            setShowEnableProdEditsModal(undefined);
          }}
        />
      )}
      <div className="flex w-full flex-col gap-2 border-b p-4 px-2">
        <div className="flex items-center justify-between gap-1 text-xs">
          {rows.length} document{rows.length !== 1 && "s"} selected
          <CopyButton
            text={
              rows.length === 1 ? stringifyValue(rows[0]) : stringifyValue(rows)
            }
            inline
            className="text-xs"
            tip={
              rows.length === 1
                ? "Copy document"
                : `Copy ${rows.length} selected documents`
            }
            tipSide="left"
          />
        </div>
        <TextInput
          id="searchFields"
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by field name"
          className="w-full"
        />
      </div>
      <div className="mt-2 scrollbar flex w-full flex-col gap-2 overflow-x-hidden overflow-y-auto p-2">
        {columns
          .filter(
            (c) =>
              c !== "*select" &&
              c.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
          )
          .sort()
          .map((column) => {
            const value = Array.from(new Set(rows.map((row) => row[column])));
            return (
              <div
                key={column}
                className="flex w-full flex-col items-center gap-1"
              >
                <div className="flex w-full items-center justify-between gap-4">
                  <div className="shrink text-xs font-medium">{column}</div>
                  {documentValidator?.type === "object" &&
                  documentValidator.value[column] ? (
                    <ValidatorTooltip
                      fieldSchema={documentValidator.value[column]}
                      columnName={column}
                    >
                      <code className="ml-auto truncate text-right text-xs text-content-tertiary">
                        {displayObjectFieldSchema(
                          documentValidator.value[column],
                        )}
                      </code>
                    </ValidatorTooltip>
                  ) : null}
                </div>
                {editingColumn === column ? (
                  <EditDocumentField
                    column={editingColumn}
                    rows={rows}
                    close={() => setEditingColumn(undefined)}
                    value={value}
                    tableName={tableName}
                    componentId={componentId}
                    validator={validator}
                    allowTopLevelUndefined={allowTopLevelUndefined}
                    shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
                  />
                ) : (
                  <div className="flex w-full gap-1">
                    <Button
                      className="flex h-[2.25rem] w-full cursor-text items-center justify-between truncate rounded-md border bg-background-secondary px-2 disabled:cursor-not-allowed"
                      variant="unstyled"
                      onClick={() => {
                        if (!areEditsAuthorized) {
                          onAuthorizeEdits &&
                            setShowEnableProdEditsModal(column);
                          return;
                        }
                        setEditingColumn(column);
                      }}
                      tip={
                        isInUnmountedComponent
                          ? "Cannot edit documents in an unmounted component."
                          : !canManageTable
                            ? "You do not have permission to edit data in production."
                            : undefined
                      }
                      disabled={
                        !canManageTable ||
                        isInUnmountedComponent ||
                        column.startsWith("_")
                      }
                    >
                      {value.length === 1 ? (
                        column === "_creationTime" ? (
                          <span className="truncate text-xs">
                            {new Date(value[0] as number).toLocaleString()}{" "}
                            <span className="font-mono text-content-secondary">
                              ({stringifyValue(value[0])})
                            </span>
                          </span>
                        ) : (
                          <span className="truncate font-mono text-xs">
                            {stringifyValue(value[0])}
                          </span>
                        )
                      ) : (
                        <span className="truncate font-mono text-xs text-content-secondary italic">
                          multiple values
                        </span>
                      )}
                    </Button>
                    <CopyButton
                      className="text-xs"
                      tip={`Copy '${column}' for selected documents`}
                      text={
                        value.length === 1
                          ? stringifyValue(value[0])
                          : stringifyValue(value)
                      }
                      inline
                    />
                  </div>
                )}
              </div>
            );
          })}
      </div>
    </div>
  );
}
