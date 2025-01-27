import { useMemo, useRef } from "react";

type ConvexDocument = {
  _creationTime: number;
  _id: string;
};

/**
 * Given an array of Convex Documents, return passed in elements plus elements
 * passed in previous renders.
 * Returned elements are sorted by `_creationTime` descending: latest first.
 *
 * Careful: this hook has no dependency array! If the `useQuery` producing results
 * for it has changing arguments, this hook will combine the results!
 */
export function useInMemoryDocumentCache<T extends ConvexDocument>(
  current: T[] | undefined,
  limit: number = 50,
): T[] | undefined {
  const cacheRef = useRef(new Map<string, T>());
  return useMemo(() => {
    if (current === undefined) return undefined;

    const cache = cacheRef.current;

    const onlyInCache = new Map(cache);
    for (const doc of current) {
      const id = doc._id;
      if (cache.has(id)) {
        onlyInCache.delete(id);
      } else {
        cache.set(id, doc);
      }
    }

    const arr = [...current];
    arr.push(...onlyInCache.values());
    arr.sort((a, b) => b._creationTime - a._creationTime);
    arr.splice(Math.max(limit, current.length));
    arr.sort((a, b) => b._creationTime - a._creationTime);
    return arr;
  }, [current, limit]);
}
