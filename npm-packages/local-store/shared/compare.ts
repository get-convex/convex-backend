import { IndexRangeBounds, Key } from "./types";
import { Value } from "convex/values";

// Returns -1 if k1 < k2
// Returns 0 if k1 === k2
// Returns 1 if k1 > k2
export function compareValues(k1: Value | undefined, k2: Value | undefined) {
  return compareAsTuples(makeComparable(k1), makeComparable(k2));
}

function compareAsTuples<T>(a: [number, T], b: [number, T]): number {
  if (a[0] === b[0]) {
    return compareSameTypeValues(a[1], b[1]);
  } else if (a[0] < b[0]) {
    return -1;
  }
  return 1;
}

function compareSameTypeValues<T>(v1: T, v2: T): number {
  if (v1 === undefined || v1 === null) {
    return 0;
  }
  if (
    typeof v1 === "bigint" ||
    typeof v1 === "number" ||
    typeof v1 === "boolean" ||
    typeof v1 === "string"
  ) {
    return v1 < v2 ? -1 : v1 === v2 ? 0 : 1;
  }
  if (!Array.isArray(v1) || !Array.isArray(v2)) {
    throw new Error(`Unexpected type ${v1 as any}`);
  }
  for (let i = 0; i < v1.length && i < v2.length; i++) {
    const cmp = compareAsTuples(v1[i], v2[i]);
    if (cmp !== 0) {
      return cmp;
    }
  }
  if (v1.length < v2.length) {
    return -1;
  }
  if (v1.length > v2.length) {
    return 1;
  }
  return 0;
}

// Returns an array which can be compared to other arrays as if they were tuples.
// For example, [1, null] < [2, 1n] means null sorts before all bigints
// And [3, 5] < [3, 6] means floats sort as expected
// And [7, [[5, "a"]]] < [7, [[5, "a"], [5, "b"]]] means arrays sort as expected
function makeComparable(v: Value | undefined): [number, any] {
  if (v === undefined) {
    return [0, undefined];
  }
  if (v === null) {
    return [1, null];
  }
  if (typeof v === "bigint") {
    return [2, v];
  }
  if (typeof v === "number") {
    if (isNaN(v)) {
      // Consider all NaNs to be equal.
      return [3.5, 0];
    }
    return [3, v];
  }
  if (typeof v === "boolean") {
    return [4, v];
  }
  if (typeof v === "string") {
    return [5, v];
  }
  if (v instanceof ArrayBuffer) {
    return [6, Array.from(new Uint8Array(v)).map(makeComparable)];
  }
  if (Array.isArray(v)) {
    return [7, v.map(makeComparable)];
  }
  // Otherwise, it's an POJO.
  const keys = Object.keys(v).sort();
  const pojo: Value[] = keys.map((k) => [k, v[k]!]);
  return [8, pojo.map(makeComparable)];
}

export function compareKeys(key1: Key, key2: Key): number {
  const result = _compareKeys(key1, key2);
  // onsole.log("compareKeys", key1, key2, result);
  return result;
}

function getValueAtIndex(
  v: Value[],
  index: number,
): { kind: "found"; value: Value } | undefined {
  if (index >= v.length) {
    return undefined;
  }
  return { kind: "found", value: v[index] };
}

function compareDanglingSuffix(
  shorterKeyKind: "exact" | "successor" | "predecessor",
  longerKeyKind: "exact" | "successor" | "predecessor",
  shorterKey: Key,
  longerKey: Key,
): number {
  if (shorterKeyKind === "exact" && longerKeyKind === "exact") {
    throw new Error(
      `Exact keys are not the same length:  ${JSON.stringify(
        shorterKey.value,
      )}, ${JSON.stringify(longerKey.value)}`,
    );
  }
  if (shorterKeyKind === "exact") {
    throw new Error(
      `Exact key is shorter than prefix: ${JSON.stringify(
        shorterKey.value,
      )}, ${JSON.stringify(longerKey.value)}`,
    );
  }
  if (shorterKeyKind === "predecessor" && longerKeyKind === "successor") {
    // successor is longer than predecessor, so it is bigger
    return -1;
  }
  if (shorterKeyKind === "successor" && longerKeyKind === "predecessor") {
    // successor is shorter than predecessor, so it is larger
    return 1;
  }
  if (shorterKeyKind === "predecessor" && longerKeyKind === "predecessor") {
    // predecessor of [2, 3] contains [2, 1] while predecessor of [2] doesn't, so longer predecessors are larger
    return -1;
  }
  if (shorterKeyKind === "successor" && longerKeyKind === "successor") {
    // successor of [2, 3] contains [2, 4] while successor of [2] doesn't, so longer successors are smaller
    return 1;
  }
  if (shorterKeyKind === "predecessor" && longerKeyKind === "exact") {
    return -1;
  }
  if (shorterKeyKind === "successor" && longerKeyKind === "exact") {
    return 1;
  }
  throw new Error(`Unexpected key kinds: ${shorterKeyKind}, ${longerKeyKind}`);
}

function _compareKeys(key1: Key, key2: Key): number {
  let i = 0;
  while (i < Math.max(key1.value.length, key2.value.length)) {
    const v1 = getValueAtIndex(key1.value as any, i);
    const v2 = getValueAtIndex(key2.value as any, i);
    if (v1 === undefined) {
      return compareDanglingSuffix(key1.kind, key2.kind, key1, key2);
    }
    if (v2 === undefined) {
      return -1 * compareDanglingSuffix(key2.kind, key1.kind, key2, key1);
    }
    const result = compareValues(v1.value, v2.value);
    if (result !== 0) {
      return result;
    }
    // if the prefixes are the same so far, keep going with the comparison
    i++;
  }

  if (key1.kind === key2.kind) {
    return 0;
  }

  // keys are the same length and values
  if (key1.kind === "exact") {
    if (key2.kind === "successor") {
      return -1;
    } else {
      return 1;
    }
  }
  if (key1.kind === "predecessor") {
    return -1;
  }
  if (key1.kind === "successor") {
    return 1;
  }
  throw new Error(`Unexpected key kind: ${key1.kind as any}`);
}

function testCompareKeys(
  key1: Key,
  key2: Key,
  expected: "firstBigger" | "secondBigger" | "equal",
) {
  const result = compareKeys(key1, key2);
  const expectedResult =
    expected === "firstBigger" ? 1 : expected === "secondBigger" ? -1 : 0;

  if (result !== expectedResult) {
    throw new Error(
      `compareKeys(${JSON.stringify(key1)}, ${JSON.stringify(
        key2,
      )}) = ${result}, expected ${expected}`,
    );
  }
}

export function testAllCompareKeys() {
  testCompareKeys(
    { kind: "exact", value: [2, 3] },
    { kind: "successor", value: [2] },
    "secondBigger",
  );
  testCompareKeys(
    { kind: "exact", value: [2] },
    { kind: "successor", value: [] },
    "secondBigger",
  );
  testCompareKeys(
    { kind: "predecessor", value: [2, 3] },
    { kind: "predecessor", value: [2] },
    "firstBigger",
  );
  testCompareKeys(
    { kind: "successor", value: [2, 3] },
    { kind: "successor", value: [2] },
    "secondBigger",
  );
  testCompareKeys(
    { kind: "successor", value: [2, 3] },
    { kind: "predecessor", value: [2, 3] },
    "firstBigger",
  );
  testCompareKeys(
    { kind: "successor", value: [2, 3] },
    { kind: "exact", value: [2, 3] },
    "firstBigger",
  );
  testCompareKeys(
    { kind: "predecessor", value: [2, 3] },
    { kind: "predecessor", value: [2, 3] },
    "equal",
  );
  testCompareKeys(
    { kind: "predecessor", value: [2] },
    { kind: "exact", value: [2, 3] },
    "secondBigger",
  );
}

export function minimalKey(indexRangeBounds: IndexRangeBounds): Key {
  if (indexRangeBounds.lowerBoundInclusive) {
    return { kind: "predecessor", value: indexRangeBounds.lowerBound };
  } else {
    return { kind: "successor", value: indexRangeBounds.lowerBound };
  }
}

export function maximalKey(indexRangeBounds: IndexRangeBounds): Key {
  if (indexRangeBounds.upperBoundInclusive) {
    return { kind: "successor", value: indexRangeBounds.upperBound };
  } else {
    return { kind: "predecessor", value: indexRangeBounds.upperBound };
  }
}
