import { Option } from "@commander-js/extra-typings";

/** Default `--top` for the list commands. */
export const DEFAULT_TOP = 30;

export function jsonOption() {
  return new Option("--json", "Print machine-readable JSON to stdout");
}
