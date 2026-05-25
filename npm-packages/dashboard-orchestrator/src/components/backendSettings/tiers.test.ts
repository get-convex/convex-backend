import {
  clampCustomTierResources,
  encodeCustomTier,
  parseCustomTier,
  TIERS,
  tierDefaultsForName,
} from "./tiers";

describe("backend tier helpers", () => {
  test("preset tiers no longer include max", () => {
    expect(TIERS.map((tier) => tier.name)).toEqual([
      "S4",
      "S8",
      "S16",
      "S32",
      "S64",
      "S128",
      "S256",
    ]);
  });

  test("custom tiers round-trip RAM and CPU allocation", () => {
    const tier = encodeCustomTier({ memoryMb: 12288, cpus: 6.5 });

    expect(tier).toBe("custom:12288:6.5");
    expect(parseCustomTier(tier)).toEqual({ memoryMb: 12288, cpus: 6.5 });
  });

  test("custom tier resources clamp to system maximums", () => {
    expect(
      clampCustomTierResources(
        { memoryMb: 65536, cpus: 32 },
        { totalMemoryMb: 49152, totalCpus: 16 },
      ),
    ).toEqual({ memoryMb: 49152, cpus: 16 });
  });

  test("tier defaults mirror orchestrator knob tuning", () => {
    expect(tierDefaultsForName("S16")).toMatchObject({
      UDF_CACHE_MAX_SIZE: "104857600",
      FUNRUN_INDEX_CACHE_SIZE: "50000000",
      RUNTIME_WORKER_THREADS: "2",
      POSTGRES_MAX_CONNECTIONS: "128",
    });

    expect(tierDefaultsForName("custom:12288:6.5")).toMatchObject({
      RUNTIME_WORKER_THREADS: "7",
      POSTGRES_MAX_CONNECTIONS: "768",
    });
  });
});
