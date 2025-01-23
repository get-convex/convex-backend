import {
  CopyButton,
  Button,
  useNents,
  SchemaJson,
  stringifyValue,
} from "dashboard-common";
import { GenericDocument } from "convex/server";
import { useState } from "react";
import { TextInput } from "../../../../elements/TextInput";
import { ProductionEditsConfirmationDialog } from "../../../../elements/ProductionEditsConfirmationDialog";
import { EditDocumentField } from "./EditDocumentField";
import {
  documentValidatorForTable,
  validatorForColumn,
} from "./utils/validators";

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
    <div className="flex h-full w-full min-w-[10rem] flex-col items-start overflow-y-hidden rounded-r border-l bg-background-secondary">
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
      <div className="flex w-full flex-col gap-2 border-b p-4">
        <div className="ml-auto flex items-center gap-1 text-xs text-content-tertiary">
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
      <div className="mt-2 flex w-full flex-col gap-2 overflow-y-auto overflow-x-hidden px-4 py-2 scrollbar">
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
                <div className="flex w-full justify-between text-xs">
                  <code>{column}</code>
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
                      className="flex h-[2.25rem] w-full cursor-text items-center justify-between truncate rounded border px-2 disabled:cursor-not-allowed"
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
                        <span className="truncate font-mono text-xs italic text-content-secondary">
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
