import { ValidatorJSON, Value } from "convex/values";
import { useState } from "react";
import isEqual from "lodash/isEqual";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/lib/values";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";
import { useTableDensity } from "@common/features/data/lib/useTableDensity";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";
import { isInCommonUTCTimestampRange } from "@common/features/data/lib/helpers";

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

  const isTimestampLike =
    typeof editedValue === "number" && isInCommonUTCTimestampRange(editedValue);

  const [showAsDate, setShowAsDate] = useState(isTimestampLike);

  const [innerText, setInnerText] = useState<string | undefined>(undefined);
  const { densityValues } = useTableDensity();

  return (
    // eslint-disable-next-line jsx-a11y/no-static-element-interactions
    <div
      className="flex h-full w-full flex-col items-end justify-between gap-1 border border-border-selected bg-background-secondary text-xs text-content-primary"
      style={{
        paddingLeft: densityValues.paddingX,
        paddingTop: densityValues.paddingY,
      }}
      onKeyDown={(e) => {
        if (isTimestampLike && e.ctrlKey && e.shiftKey && e.key === "D") {
          setShowAsDate(!showAsDate);
        }
      }}
    >
      {/* Monaco editor cannot show a placeholder, so render our own. */}
      {!innerText && editedValue === UNDEFINED_PLACEHOLDER && !error && (
        <div
          className="pointer-events-none absolute z-50 font-mono text-xs text-content-secondary italic"
          data-testid="undefined-placeholder"
          style={{
            top: densityValues.paddingY + 2,
            left: densityValues.paddingX,
          }}
        >
          unset
        </div>
      )}
      {showAsDate && isTimestampLike && typeof editedValue === "number" ? (
        <div className="w-full">
          <DateTimePicker
            date={new Date(editedValue as number)}
            onChange={(date) => setEditedValue(date.getTime())}
            className="w-fit rounded-none border-none p-0 pt-px pb-[1.1875rem] font-mono text-xs"
            mode="text-only"
            onError={setError}
            onKeyDown={(e, date) => {
              if (e.key === "Enter") {
                if (date === undefined) {
                  // User cleared the input - check if undefined is allowed
                  if (!allowTopLevelUndefined) {
                    setError("This field is required and cannot be unset");
                    return;
                  }
                  setEditedValue(UNDEFINED_PLACEHOLDER);
                  void saveEditedValue(UNDEFINED_PLACEHOLDER);
                } else {
                  setEditedValue(date.getTime());
                  void saveEditedValue(date.getTime());
                }
              }
            }}
          />
        </div>
      ) : (
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
      )}
      <div className="mr-2 flex w-full flex-wrap items-center justify-between gap-2 pb-1 pl-2">
        {isTimestampLike && (
          <div className="min-w-fit text-xs text-content-secondary">
            <KeyboardShortcut
              value={["Ctrl", "Shift", "D"]}
              className="font-semibold"
            />{" "}
            to show as {showAsDate ? "number" : "date"}
          </div>
        )}
        <div className="ml-auto">
          {error ? (
            <p
              className="w-full font-mono text-xs break-all text-content-errorSecondary"
              role="alert"
            >
              {`${error.slice(0, 80)}${error.length > 80 ? "..." : ""}`}
            </p>
          ) : (
            <span className="flex gap-4 text-xs text-content-secondary">
              <div>
                <KeyboardShortcut value={["Esc"]} className="font-semibold" />{" "}
                to cancel
              </div>
              <div>
                <KeyboardShortcut
                  value={["Return"]}
                  className="font-semibold"
                />{" "}
                to save
              </div>
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
