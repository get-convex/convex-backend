// Mirror of crates/orchestrator/src/provisioner/tiers.rs. The orchestrator
// is the source of truth — the dashboard only uses this for capacity
// projection math + summary labels. The /api/dashboard/host_capacity
// endpoint reflects the orchestrator's view of the allocation, so any
// drift here is bounded to per-tier copy in the UI.
export type Tier = {
  name: "S4" | "S8" | "S16" | "S32";
  memoryMb: number;
  cpus: number;
};

export const TIERS: Tier[] = [
  { name: "S4", memoryMb: 1024, cpus: 0.5 },
  { name: "S8", memoryMb: 2048, cpus: 1.0 },
  { name: "S16", memoryMb: 4096, cpus: 2.0 },
  { name: "S32", memoryMb: 8192, cpus: 4.0 },
];

export const DEFAULT_TIER: Tier["name"] = "S16";

export function lookupTier(name: string): Tier | undefined {
  return TIERS.find((t) => t.name === name);
}
