import {
  functionIdentifierFromValue,
  functionIdentifierValue,
} from "../../../lib/functions/generateFileTree";
import { UdfLog } from "../../../lib/useLogs";

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
  selectedFunctions: string[],
): boolean {
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
    logTypes: string[];
    functions: string[];
    selectedFunctions: string[];
    filter: string;
  },
  logs?: UdfLog[],
) {
  const statuses = logTypes.filter((l) => l === "success" || l === "failure");
  const levels = logTypes.filter((l) => l !== "success" && l !== "failure");
  const functionsWithoutId = functions.map(stripComponentId);
  const selectedFunctionsWithoutId = selectedFunctions.map(stripComponentId);
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
