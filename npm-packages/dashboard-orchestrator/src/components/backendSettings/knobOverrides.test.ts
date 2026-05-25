import type { KnobEntry } from "../../lib/orchestratorApi";
import {
  clearVisibleOverrides,
  knobRowState,
  visibleOverrideCount,
} from "./knobOverrides";

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

describe("backend knob override helpers", () => {
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

  test("clear visible overrides preserves hidden infrastructure overrides", () => {
    expect(
      clearVisibleOverrides(
        {
          CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
          RUNTIME_WORKER_THREADS: "8",
          ACTIONS_USER_TIMEOUT_SECS: "120",
        },
        registry,
      ),
    ).toEqual({
      CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
    });
  });

  test("row state displays tier and upstream defaults when not overridden", () => {
    expect(
      knobRowState(registry[0], {}, { RUNTIME_WORKER_THREADS: "2" }),
    ).toEqual({
      source: "tier",
      effectiveValue: "2",
      overrideValue: "",
    });

    expect(knobRowState(registry[1], {}, {})).toEqual({
      source: "default",
      effectiveValue: "600",
      overrideValue: "",
    });
  });

  test("empty strings are still explicit overrides", () => {
    expect(
      knobRowState(registry[1], { ACTIONS_USER_TIMEOUT_SECS: "" }, {}),
    ).toEqual({
      source: "override",
      effectiveValue: "",
      overrideValue: "",
    });
  });
});
