import {
  clampCustomTierResources,
  encodeCustomTier,
  parseCustomTier,
  TIERS,
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
});
