import { ValidatorJSON, Value } from "convex/values";
import { useState } from "react";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/lib/values";
import { cn } from "@ui/cn";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";

export function ObjectEditorWithPlaceholder({
  value,
  onChangeHandler,
  path,
  autoFocus = false,
  className = "",
  enabled,
  onApplyFilters,
  handleError,
  documentValidator,
  shouldSurfaceValidatorErrors,
}: {
  value: any;
  onChangeHandler: (value?: Value) => void;
  path: string;
  autoFocus?: boolean;
  className?: string;
  enabled: boolean;
  onApplyFilters: () => void;
  handleError: (errors: string[]) => void;
  documentValidator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
}) {
  const [innerText, setInnerText] = useState("");

  return (
    <>
      {enabled && innerText === "" && value === UNDEFINED_PLACEHOLDER && (
        <div
          className="pointer-events-none absolute z-50 font-mono text-xs text-content-secondary italic"
          data-testid="undefined-placeholder"
          style={{
            marginTop: "5px",
            marginLeft: "11px",
          }}
        >
          unset
        </div>
      )}
      <ObjectEditor
        key={path}
        className={cn(
          "w-full min-w-4 border focus-within:border focus-within:border-border-selected",
          enabled && "border-l-transparent",
          className,
        )}
        editorClassname={cn(
          "mt-0 rounded-sm bg-background-secondary px-2 py-1 text-xs",
          className,
        )}
        allowTopLevelUndefined
        size="sm"
        disableFolding
        defaultValue={value === UNDEFINED_PLACEHOLDER ? undefined : value}
        onChange={onChangeHandler}
        onError={handleError}
        path={path}
        autoFocus={autoFocus}
        disableFind
        saveAction={onApplyFilters}
        enterSaves
        mode="editField"
        validator={documentValidator}
        shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
        disabled={!enabled}
        onChangeInnerText={setInnerText}
      />
    </>
  );
}
