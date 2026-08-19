import { db } from "convex:runtime";

export async function roundTrip(key: string, value: unknown) {
  await db.set(key, value);
  return db.get(key);
}

export async function fanout(keys: string[]) {
  return Promise.all(keys.map((key) => db.get(key)));
}

export async function sequential(keys: string[]) {
  const results = [];
  for (const key of keys) {
    results.push(await db.get(key));
  }
  return results;
}

export async function failAfterAwait(key: string) {
  await db.get(key);
  throw new Error(`boom:${key}`);
}
