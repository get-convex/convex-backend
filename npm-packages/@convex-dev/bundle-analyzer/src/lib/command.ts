import { Option } from "@commander-js/extra-typings";

export function jsonOption() {
  return new Option("--json", "Print machine-readable JSON to stdout");
}
