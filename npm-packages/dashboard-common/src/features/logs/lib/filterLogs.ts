import {
  functionIdentifierFromValue,
  functionIdentifierValue,
} from "@common/lib/functions/generateFileTree";
import { UdfLog } from "@common/lib/useLogs";
import { MultiSelectValue } from "@ui/MultiSelectCombobox";

export const ALL_LEVELS = ["DEBUG", "INFO", "WARN", "ERROR", "FAILURE"];

function filterEntryForLogLevels(entry: UdfLog, levels: string[]): boolean {
  if (entry.kind !== "log") return true;
  const out = entry.output;

  return levels.some(
    (level) =>
      (level === "ERROR" && out.level === "FAILURE") ||
      (level === "INFO" && out.level === "LOG") ||
      out.level === level,
  );
}

function filterEntryForStatuses(entry: UdfLog, statuses: string[]): boolean {
  if (entry.kind !== "outcome") return true;
  return statuses.includes(entry.outcome.status);
}

function filterEntryForLogLines(entry: UdfLog, levels: string[]): boolean {
  if (entry.kind !== "log") return true;
  const levelsArray = Array.from(levels);
  const out = entry.output;
  return levelsArray.some(
    (level) =>
      (level === "ERROR" && out.level === "FAILURE") ||
      (level === "INFO" && out.level === "LOG") ||
      out.level === level,
  );
}

function filterEntryForRawText(entry: UdfLog, text: string): boolean {
  const requestIdMatches = text.includes(entry.requestId);

  const lowerCaseText = text.toLocaleLowerCase();
  return (
    requestIdMatches ||
    entry.call.toLocaleLowerCase().includes(lowerCaseText) ||
    (entry.kind === "log"
      ? entry.output.messages
          .join(" ")
          .toLocaleLowerCase()
          .includes(lowerCaseText)
      : !!entry.error?.toLocaleLowerCase().includes(lowerCaseText))
  );
}

function stripComponentId(call: string) {
  const f = functionIdentifierFromValue(call);
  return functionIdentifierValue(f.identifier, f.componentPath);
}

function filterEntryForFunction(
  entry: UdfLog,
  functions: string[],
  selectedFunctions: string[] | "all",
): boolean {
  // If "all" is selected, return true for all entries
  if (selectedFunctions === "all") {
    return true;
  }

  const entryFunction =
    (entry.kind === "log" ? entry.output.subfunction : undefined) ?? entry.call;
  return (
    selectedFunctions.includes(entryFunction) ||
    (!functions.includes(entryFunction) &&
      selectedFunctions.includes(functionIdentifierValue("_other")))
  );
}

export function filterLogs(
  {
    logTypes,
    functions,
    selectedFunctions,
    filter,
  }: {
    logTypes: MultiSelectValue;
    functions: string[];
    selectedFunctions: MultiSelectValue;
    filter: string;
  },
  logs?: UdfLog[],
) {
  // Handle logTypes "all" case
  const logTypesArray =
    logTypes === "all"
      ? ["success", "failure", "DEBUG", "INFO", "WARN", "ERROR"]
      : logTypes;

  const statuses = logTypesArray.filter(
    (l) => l === "success" || l === "failure",
  );
  const levels = logTypesArray.filter(
    (l) => l !== "success" && l !== "failure",
  );
  const functionsWithoutId = functions.map(stripComponentId);

  // Handle selectedFunctions "all" case
  const selectedFunctionsWithoutId =
    selectedFunctions === "all"
      ? "all"
      : selectedFunctions.map(stripComponentId);

  return logs?.filter(
    (entry) =>
      filterEntryForFunction(
        entry,
        functionsWithoutId,
        selectedFunctionsWithoutId,
      ) &&
      filterEntryForStatuses(entry, statuses) &&
      filterEntryForLogLevels(entry, levels) &&
      filterEntryForLogLines(entry, levels) &&
      (filter ? filterEntryForRawText(entry, filter) : true),
  );
}
