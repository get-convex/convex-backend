import { console, crypto, db, now } from "convex:runtime";

export async function writeThenRead(key: string, value: unknown) {
  await db.set(key, value);
  return db.get(key);
}

export async function removeThenRead(key: string, value: unknown) {
  await db.set(key, value);
  await db.delete(key);
  return db.get(key);
}

// Reads back a document large enough to exercise host-sized results. Returns a
// summary rather than the document itself: what is under test is the syscall
// boundary, not `invoke`'s own packed-pointer return.
export async function roundTripLargeDocument(key: string, sizeBytes: number) {
  const chunk = "x".repeat(1024);
  const blocks = Array.from(
    { length: Math.ceil(sizeBytes / chunk.length) },
    (_unused, index) => `${index}:${chunk}`,
  );

  const written = { key, blocks };
  await db.set(key, written);
  const read = await db.get(key);

  // The host stores documents in a sorted JSON map, so a round trip preserves
  // values but not key order. Compare field by field.
  return {
    blocks: read.blocks.length,
    bytes: JSON.stringify(read).length,
    identical:
      read.key === written.key &&
      read.blocks.length === blocks.length &&
      read.blocks.every(
        (block: string, index: number) => block === blocks[index],
      ),
  };
}

export async function read(key: string) {
  return db.get(key);
}

export async function logAndDescribe(key: string) {
  console.log("reading", key);
  return {
    key,
    now: now(),
    uuid: crypto.randomUUID(),
    value: await db.get(key),
  };
}
