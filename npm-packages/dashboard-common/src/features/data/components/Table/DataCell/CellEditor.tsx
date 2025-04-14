import { ValidatorJSON, Value } from "convex/values";
import { useState } from "react";
import isEqual from "lodash/isEqual";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";
import { useTableDensity } from "@common/features/data/lib/useTableDensity";

export type CellEditorProps = {
  value?: Value;
  defaultValue?: Value;
  onStopEditing: () => void;
  onSave(value?: Value): Promise<any>;
  validator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
  allowTopLevelUndefined?: boolean;
};

export function CellEditor({
  value,
  defaultValue,
  onStopEditing,
  onSave,
  validator,
  shouldSurfaceValidatorErrors,
  allowTopLevelUndefined,
}: CellEditorProps) {
  const [path] = useState(Math.random());
  const [error, setError] = useState<string | undefined>(undefined);
  const saveEditedValue = async (editedValue?: Value) => {
    if (editedValue === undefined || error) {
      return;
    }

    onStopEditing();
    if (isEqual(value, editedValue)) {
      return;
    }
    await onSave(editedValue);
  };

  const [editedValue, setEditedValue] = useState(
    defaultValue === undefined ? value : defaultValue,
  );
  const [innerText, setInnerText] = useState<string | undefined>(undefined);
  const { densityValues } = useTableDensity();

  return (
    <div
      className="flex h-full flex-col items-end justify-between gap-1 border border-border-selected bg-background-secondary text-xs text-content-primary"
      style={{
        paddingLeft: densityValues.paddingX,
        paddingTop: densityValues.paddingY,
      }}
    >
      {/* Monaco editor cannot show a placeholder, so render our own. */}
      {!innerText && editedValue === UNDEFINED_PLACEHOLDER && !error && (
        <div
          className="pointer-events-none absolute z-50 font-mono text-xs italic text-content-secondary"
          data-testid="undefined-placeholder"
          style={{
            top: densityValues.paddingY + 2,
            left: densityValues.paddingX,
          }}
        >
          unset
        </div>
      )}
      <ObjectEditor
        validator={validator}
        shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
        padding={false}
        autoFocus
        enterSaves
        saveAction={() => saveEditedValue(editedValue)}
        disableFind
        defaultValue={
          defaultValue === UNDEFINED_PLACEHOLDER ? undefined : editedValue
        }
        onChange={setEditedValue}
        onChangeInnerText={setInnerText}
        onError={(errors) =>
          setError(errors.length > 0 ? errors[0] : undefined)
        }
        path={path.toString()}
        disableFolding
        className="border-none"
        allowTopLevelUndefined={allowTopLevelUndefined}
        mode="editField"
        fixedOverflowWidgets={false}
      />
      <div className="mr-2">
        {error ? (
          <p
            className="w-full break-all font-mono text-xs text-content-errorSecondary"
            role="alert"
          >
            {`${error.slice(0, 80)}${error.length > 80 ? "..." : ""}`}
          </p>
        ) : (
          <span className="flex gap-4 text-sm text-content-secondary">
            <div>
              <KeyboardShortcut value={["Esc"]} className="font-semibold" /> to
              cancel
            </div>
            <div>
              <KeyboardShortcut value={["Return"]} className="font-semibold" />{" "}
              to save
            </div>
          </span>
        )}
      </div>
    </div>
  );
}
