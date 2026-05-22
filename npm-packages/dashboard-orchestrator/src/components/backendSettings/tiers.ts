// Mirror of crates/orchestrator/src/provisioner/tiers.rs. The orchestrator
// is the source of truth; the dashboard uses this for allocation projection
// and summary labels only.
export type TierResources = {
  memoryMb: number;
  cpus: number;
};

export type Tier = {
  name: "S4" | "S8" | "S16" | "S32" | "S64" | "S128" | "S256";
} & TierResources;

export const TIERS: Tier[] = [
  { name: "S4", memoryMb: 1024, cpus: 0.5 },
  { name: "S8", memoryMb: 2048, cpus: 1.0 },
  { name: "S16", memoryMb: 4096, cpus: 2.0 },
  { name: "S32", memoryMb: 8192, cpus: 4.0 },
  { name: "S64", memoryMb: 16384, cpus: 8.0 },
  { name: "S128", memoryMb: 32768, cpus: 16.0 },
  { name: "S256", memoryMb: 65536, cpus: 32.0 },
];

export const DEFAULT_TIER: Tier["name"] = "S16";

export function lookupTier(name: string): Tier | undefined {
  return TIERS.find((t) => t.name === name);
}

export function isCustomTier(name: string): boolean {
  return parseCustomTier(name) !== undefined;
}

export function parseCustomTier(name: string): TierResources | undefined {
  const parts = name.split(":");
  if (parts.length !== 3 || parts[0] !== "custom") {
    return undefined;
  }
  const memoryMb = Number(parts[1]);
  const cpus = Number(parts[2]);
  if (
    !Number.isInteger(memoryMb) ||
    memoryMb <= 0 ||
    !Number.isFinite(cpus) ||
    cpus <= 0
  ) {
    return undefined;
  }
  return { memoryMb, cpus };
}

export function encodeCustomTier(resources: TierResources): string {
  const memoryMb = Math.max(1, Math.round(resources.memoryMb));
  return `custom:${memoryMb}:${formatCpu(resources.cpus)}`;
}

export function tierResourcesForName(name: string): TierResources | undefined {
  return lookupTier(name) ?? parseCustomTier(name);
}

export function clampCustomTierResources(
  resources: TierResources,
  capacity: { totalMemoryMb: number; totalCpus: number },
): TierResources {
  return {
    memoryMb: Math.min(
      Math.max(1, Math.round(resources.memoryMb)),
      Math.max(1, Math.floor(capacity.totalMemoryMb)),
    ),
    cpus: Math.min(
      Math.max(0.1, resources.cpus),
      Math.max(0.1, capacity.totalCpus),
    ),
  };
}

export function formatTierResources(resources: TierResources): string {
  return `${formatMemory(resources.memoryMb)} · ${formatCpu(resources.cpus)} CPUs`;
}

function formatMemory(memoryMb: number): string {
  const gb = memoryMb / 1024;
  return `${Number(gb.toFixed(gb >= 10 ? 1 : 2))} GB`;
}

function formatCpu(cpus: number): string {
  return Number(cpus.toFixed(2)).toString();
}
