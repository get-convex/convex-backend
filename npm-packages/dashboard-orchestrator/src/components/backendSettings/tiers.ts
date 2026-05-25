// Mirror of crates/orchestrator/src/provisioner/tiers.rs. The orchestrator
// is the source of truth; the dashboard uses this for allocation projection,
// summary labels, and displayed tier-default knob values.
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

const BASE_MEMORY_MB = 4096;
const BASE_CPUS = 2.0;
const MIN_KNOB_SCALE = 0.25;
const KNOB_HEADROOM_MULTIPLIER = 3.0;
const LEGACY_MAX_KNOB_MEMORY_MB = 131_072;
const LEGACY_MAX_KNOB_CPUS = 64.0;

const BASE_KNOB_DEFAULTS: Record<string, number> = {
  RUNTIME_WORKER_THREADS: 4,
  UDF_CACHE_MAX_SIZE: 104_857_600,
  FUNRUN_INDEX_CACHE_SIZE: 50_000_000,
  FUNRUN_MODULE_CACHE_SIZE: 250_000_000,
  FUNRUN_CODE_CACHE_SIZE: 500_000_000,
  FUNRUN_MAX_ISOLATE_WORKERS: 128,
  HTTP_SERVER_TCP_BACKLOG: 256,
  HTTP_SERVER_MAX_CONCURRENT_REQUESTS: 1024,
  APPLICATION_MAX_CONCURRENT_QUERIES: 16,
  APPLICATION_MAX_CONCURRENT_MUTATIONS: 16,
  APPLICATION_MAX_CONCURRENT_V8_ACTIONS: 64,
  APPLICATION_MAX_CONCURRENT_NODE_ACTIONS: 64,
  MAX_CONCURRENT_ACTION_OPS: 8,
  COMMITTER_QUEUE_SIZE: 128,
  MAX_BYTES_WRITTEN_PER_SECOND: 4_194_304,
  POSTGRES_MAX_CONNECTIONS: 128,
};

export function tierDefaultsForName(name: string): Record<string, string> {
  const unbounded = name === "max";
  const resources = unbounded
    ? { memoryMb: LEGACY_MAX_KNOB_MEMORY_MB, cpus: LEGACY_MAX_KNOB_CPUS }
    : tierResourcesForName(name);
  if (!resources) {
    return {};
  }

  const scale = resourceKnobScale(resources.memoryMb, resources.cpus);
  return Object.fromEntries(
    Object.entries(BASE_KNOB_DEFAULTS).map(([key, value]) => {
      const scaled =
        key === "RUNTIME_WORKER_THREADS"
          ? runtimeWorkerThreadsForResources(resources.cpus, unbounded)
          : key === "POSTGRES_MAX_CONNECTIONS"
            ? Math.ceil(
                value *
                  postgresConnectionScale(resources.memoryMb, resources.cpus),
              )
            : Math.ceil(value * scale);
      return [key, String(scaled)];
    }),
  );
}

function resourceKnobScale(memoryMb: number, cpus: number): number {
  const raw = rawResourceScale(memoryMb, cpus);
  if (raw > 1.0) {
    return Math.ceil(raw * KNOB_HEADROOM_MULTIPLIER);
  }
  if (raw < 1.0) {
    return Math.min(raw * KNOB_HEADROOM_MULTIPLIER, 1.0);
  }
  return raw;
}

function rawResourceScale(memoryMb: number, cpus: number): number {
  return Math.max(memoryMb / BASE_MEMORY_MB, cpus / BASE_CPUS, MIN_KNOB_SCALE);
}

function postgresConnectionScale(memoryMb: number, cpus: number): number {
  const raw = rawResourceScale(memoryMb, cpus);
  if (raw > 1.0) {
    return Math.ceil(Math.sqrt(raw) * KNOB_HEADROOM_MULTIPLIER);
  }
  if (raw < 1.0) {
    return Math.min(raw * KNOB_HEADROOM_MULTIPLIER, 1.0);
  }
  return raw;
}

function runtimeWorkerThreadsForResources(
  cpus: number,
  unbounded: boolean,
): number {
  if (unbounded) {
    return 0;
  }
  return Math.min(Math.max(Math.ceil(cpus), 1), 64);
}
