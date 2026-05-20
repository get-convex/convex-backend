// Mirror of crates/orchestrator/src/provisioner/tiers.rs. The orchestrator
// is the source of truth — the dashboard only uses this for capacity
// projection math + summary labels. The /api/dashboard/host_capacity
// endpoint reflects the orchestrator's view of the allocation, so any
// drift here is bounded to per-tier copy in the UI.
export type Tier = {
  name: "S4" | "S8" | "S16" | "S32" | "S64" | "S128" | "S256" | "max";
  memoryMb: number;
  cpus: number;
  unbounded?: boolean;
};

export const TIERS: Tier[] = [
  { name: "S4", memoryMb: 1024, cpus: 0.5 },
  { name: "S8", memoryMb: 2048, cpus: 1.0 },
  { name: "S16", memoryMb: 4096, cpus: 2.0 },
  { name: "S32", memoryMb: 8192, cpus: 4.0 },
  { name: "S64", memoryMb: 16384, cpus: 8.0 },
  { name: "S128", memoryMb: 32768, cpus: 16.0 },
  { name: "S256", memoryMb: 65536, cpus: 32.0 },
  { name: "max", memoryMb: 0, cpus: 0, unbounded: true },
];

export const DEFAULT_TIER: Tier["name"] = "S16";

export function lookupTier(name: string): Tier | undefined {
  return TIERS.find((t) => t.name === name);
}
