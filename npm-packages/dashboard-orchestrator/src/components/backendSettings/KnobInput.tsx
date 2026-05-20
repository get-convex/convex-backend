import { TextInput } from "@ui/TextInput";
import type { KnobEntry } from "../../lib/orchestratorApi";

function inputType(knob: KnobEntry): "bool" | "number" | "string" {
  if (knob.envVar === "UDF_USE_FUNRUN" || knob.envVar.endsWith("_ENABLED")) {
    return "bool";
  }
  if (
    knob.envVar.endsWith("_SECONDS") ||
    knob.envVar.endsWith("_SECS") ||
    knob.envVar.endsWith("_MS") ||
    knob.envVar.endsWith("_BYTES") ||
    knob.envVar.endsWith("_SIZE") ||
    knob.envVar.includes("_MAX_") ||
    knob.envVar.includes("_MIN_") ||
    knob.envVar.includes("PERCENT") ||
    knob.envVar.includes("RETRIES") ||
    knob.envVar.includes("PAGE_SIZE") ||
    knob.envVar.includes("WORKERS") ||
    knob.envVar.includes("THREADS") ||
    knob.envVar.endsWith("_USAGE") ||
    knob.envVar.endsWith("_DELAY")
  ) {
    return "number";
  }
  return "string";
}

export function KnobInput({
  knob,
  value,
  onChange,
}: {
  knob: KnobEntry;
  value: string;
  onChange: (next: string) => void;
}) {
  const type = inputType(knob);
  if (type === "bool") {
    return (
      <select
        value={value || "false"}
        onChange={(e) => onChange(e.target.value)}
        className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
      >
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    );
  }
  return (
    <TextInput
      id={`knob-${knob.envVar}`}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={type === "number" ? "0" : ""}
    />
  );
}
