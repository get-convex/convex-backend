import { useMutation } from "convex/react";
import { GenericDocument } from "convex/server";
import { useId, useState } from "react";
import udfs from "udfs";
import { ValidatorJSON, Value } from "convex/values";
import isEqual from "lodash/isEqual";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";
import { Button } from "@common/elements/Button";

export function EditDocumentField({
  column,
  rows,
  close,
  value,
  tableName,
  componentId,
  validator,
  shouldSurfaceValidatorErrors,
  allowTopLevelUndefined,
}: {
  column: string;
  rows: GenericDocument[];
  close: () => void;
  value: Value[];
  tableName: string;
  componentId: string | null;
  validator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
  allowTopLevelUndefined?: boolean;
}) {
  const [editedValue, setEditedValue] = useState<Value | undefined>(
    value.length === 1 ? value[0] : UNDEFINED_PLACEHOLDER,
  );
  const [editError, setEditError] = useState<string | undefined>(undefined);
  const patchDocumentsFields = useMutation(udfs.patchDocumentsFields.default);
  const [innerText, setInnerText] = useState<string | undefined>(undefined);
  const disabled =
    !!editError ||
    (value.length === 1 ? isEqual(editedValue, value[0]) : !innerText);

  const save = async () => {
    if (disabled) {
      return;
    }
    try {
      await patchDocumentsFields({
        ids: rows.map((r) => r._id as string),
        fields: { [column]: editedValue },
        table: tableName,
        componentId,
      });
      setEditError(undefined);
      setEditedValue(undefined);
      close();
    } catch (e: any) {
      setEditError(e.message);
    }
  };
  return (
    <form
      className="relative flex w-full flex-col gap-1"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      {/* Monaco editor cannot show a placeholder, so render our own. */}
      {!innerText && editedValue === UNDEFINED_PLACEHOLDER && (
        <div className="absolute left-2.5 top-2.5 z-50 select-none font-mono text-xs italic text-content-secondary">
          {value.length > 1 ? "enter a value to save" : "unset"}
        </div>
      )}
      <ObjectEditor
        saveAction={save}
        autoFocus
        defaultValue={value.length === 1 ? value[0] : undefined}
        path={`fieldEditor-${column}-${useId().replace(":", "_")}`}
        onChange={setEditedValue}
        onChangeInnerText={setInnerText}
        onError={(errors) => {
          errors.length > 0 ? setEditError(errors[0]) : setEditError(undefined);
        }}
        disableFolding
        className="border-border-selected pl-2"
        validator={validator}
        shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
        mode="editField"
        allowTopLevelUndefined={allowTopLevelUndefined}
      />
      {editError && (
        <p
          className="overflow-y-auto truncate font-mono text-xs text-content-errorSecondary"
          role="alert"
        >
          {editError}
        </p>
      )}
      <div className="flex w-full items-center justify-between gap-1">
        {rows.length > 1 && (
          <span className="text-xs text-content-secondary">
            Changes apply to all selected documents.
          </span>
        )}
        <div className="ml-auto flex gap-2">
          <Button
            size="xs"
            onClick={() => {
              close();
            }}
            variant="neutral"
          >
            Cancel
          </Button>
          <Button
            size="xs"
            type="submit"
            disabled={disabled}
            tip={editError ? "Fix the errors above to continue" : undefined}
          >
            Save
          </Button>
        </div>
      </div>
    </form>
  );
}
