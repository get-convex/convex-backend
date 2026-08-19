import { sleep } from "convex:runtime";

export async function sleepFor(ms: number) {
  await sleep(ms);
  return { sleptMs: ms };
}

export async function sleepPair(ms: number) {
  await Promise.all([sleep(ms), sleep(ms)]);
  return { sleptMs: ms, count: 2 };
}
