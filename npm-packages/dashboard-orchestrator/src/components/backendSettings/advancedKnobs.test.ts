import type { KnobEntry } from "../../lib/orchestratorApi";
import {
  advancedKnobRowState,
  filterAdvancedKnobs,
  visibleOverrideCount,
} from "./advancedKnobs";

const registry: KnobEntry[] = [
  {
    envVar: "RUNTIME_WORKER_THREADS",
    description: "Worker threads",
    category: "RUNTIME",
    exposure: "tierTuned",
    displayName: null,
    defaultValue: "0",
  },
  {
    envVar: "ACTIONS_USER_TIMEOUT_SECS",
    description: "Action timeout",
    category: "ACTION",
    exposure: "curated",
    displayName: "Action timeout",
    defaultValue: "600",
  },
];

describe("advanced backend knob helpers", () => {
  test("visible override count ignores hidden infrastructure overrides", () => {
    expect(
      visibleOverrideCount(
        {
          CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
          CONVEX_ORCHESTRATOR_DATABASE_MODE: "sidecar",
          CONVEX_ORCHESTRATOR_STORAGE_MODE: "sidecar",
        },
        registry,
      ),
    ).toBe(0);
  });

  test("overridden filter only returns registry knobs with explicit overrides", () => {
    const filtered = filterAdvancedKnobs({
      registry,
      overrides: {
        CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
        RUNTIME_WORKER_THREADS: "8",
      },
      search: "",
      category: "ALL",
      show: "overridden",
    });

    expect(filtered.map((knob) => knob.envVar)).toEqual([
      "RUNTIME_WORKER_THREADS",
    ]);
  });

  test("row state displays tier and upstream defaults when not overridden", () => {
    expect(
      advancedKnobRowState(registry[0], {}, { RUNTIME_WORKER_THREADS: "2" }),
    ).toEqual({
      source: "tier",
      effectiveValue: "2",
      overrideValue: "",
    });

    expect(advancedKnobRowState(registry[1], {}, {})).toEqual({
      source: "default",
      effectiveValue: "600",
      overrideValue: "",
    });
  });

  test("empty strings are still explicit overrides", () => {
    expect(
      advancedKnobRowState(registry[1], { ACTIONS_USER_TIMEOUT_SECS: "" }, {}),
    ).toEqual({
      source: "override",
      effectiveValue: "",
      overrideValue: "",
    });
  });
});
