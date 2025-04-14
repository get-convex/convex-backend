import difference from "lodash/difference";
import { MultiSelectCombobox } from "@ui/MultiSelectCombobox";
import { NentNameOption } from "@common/elements/NentSwitcher";
import { functionIdentifierFromValue } from "@common/lib/functions/generateFileTree";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { NENT_APP_PLACEHOLDER } from "@common/lib/useNents";

export function LogToolbar({
  functions,
  selectedFunctions,
  setSelectedFunctions,
  selectedLevels,
  setSelectedLevels,
  nents,
  selectedNents,
  setSelectedNents,
  firstItem,
  hideFunctionFilter = false,
}: {
  functions: string[];
  selectedFunctions: string[];
  setSelectedFunctions: (selectedFunctions: string[]) => void;
  selectedLevels: string[];
  setSelectedLevels: (selectedLevels: string[]) => void;
  nents?: string[];
  selectedNents: string[];
  setSelectedNents(newValue: string[]): void;
  firstItem?: React.ReactNode;
  hideFunctionFilter?: boolean;
}) {
  return (
    <div className="flex w-full flex-wrap items-center justify-end gap-2">
      {firstItem}
      {nents && (
        <div className="min-w-[9.5rem]">
          <MultiSelectCombobox
            options={nents}
            selectedOptions={selectedNents}
            setSelectedOptions={selectNentOption({
              selectedNents,
              setSelectedNents,
              functions,
              selectedFunctions,
              setSelectedFunctions,
            })}
            unit="component"
            unitPlural="components"
            label="Components"
            labelHidden
            Option={NentNameOption}
          />
        </div>
      )}
      {!hideFunctionFilter && (
        <div className="min-w-[9.5rem]">
          <MultiSelectCombobox
            options={functionsForSelectedNents(selectedNents, functions)}
            selectedOptions={functionsForSelectedNents(
              selectedNents,
              selectedFunctions,
            )}
            processFilterOption={(option) => {
              const id = functionIdentifierFromValue(option);
              return id.componentPath
                ? `${id.componentPath}/${id.identifier}`
                : id.identifier;
            }}
            setSelectedOptions={setSelectedFunctions}
            unit="function"
            unitPlural="functions"
            label="Functions"
            labelHidden
            Option={FunctionNameOption}
          />
        </div>
      )}
      <div className="min-w-[9.5rem]">
        <MultiSelectCombobox
          options={["success", "failure", "DEBUG", "INFO", "WARN", "ERROR"]}
          selectedOptions={selectedLevels}
          setSelectedOptions={setSelectedLevels}
          disableSearch
          unit="log types"
          unitPlural="log types"
          label="Log Types"
          labelHidden
          Option={({ label }) => (
            <div className="flex w-full justify-between">
              {label === "INFO" ? "log / info " : label.toLowerCase()}{" "}
            </div>
          )}
        />
      </div>
    </div>
  );
}

export const selectNentOption =
  ({
    selectedNents,
    functions,
    selectedFunctions,
    setSelectedFunctions,
    setSelectedNents,
  }: {
    selectedNents: string[];
    functions: string[];
    selectedFunctions: string[];
    setSelectedFunctions: (f: string[]) => void;
    setSelectedNents: (f: string[]) => void;
  }) =>
  (newNents: string[]) => {
    const addedNents = difference(newNents, selectedNents);

    const newFunctions = functionsForSelectedNents(addedNents, functions);

    const retainedFunctions = functionsForSelectedNents(
      newNents,
      selectedFunctions,
    );

    setSelectedFunctions(
      Array.from(new Set([...newFunctions, ...retainedFunctions])),
    );
    setSelectedNents(newNents);
  };

export const functionsForSelectedNents = (
  nents: string[],
  functions: string[],
) =>
  functions.filter((f) => {
    const functionIdentifier = functionIdentifierFromValue(f);
    return nents.some((nent) =>
      nent === NENT_APP_PLACEHOLDER
        ? functionIdentifier.componentPath === undefined
        : nent === functionIdentifier.componentPath,
    );
  });
