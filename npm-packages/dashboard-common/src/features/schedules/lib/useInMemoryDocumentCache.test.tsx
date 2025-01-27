import { renderHook } from "@testing-library/react";
import { Doc, Id } from "system-udfs/convex/_generated/dataModel";
import { useInMemoryDocumentCache } from "./useInMemoryDocumentCache";

type NameDoc = Pick<Doc<"_cron_jobs">, "_creationTime" | "_id" | "name">;
let nextCreationTime = 1;
let nextId = 1;
function nameDoc() {
  const id = `id-${nextId++}`;
  return {
    name: id,
    _id: id as Id<"_cron_jobs">,
    _creationTime: nextCreationTime++,
  };
}

// Each successively created NameDoc has a later `_creationTime`.
const d1 = nameDoc();
const d2 = nameDoc();
const d3 = nameDoc();
const d4 = nameDoc();
const d5 = nameDoc();

test("Adding documents", () => {
  let queryResult: NameDoc[] | undefined;

  // Cache initially returns inputs.
  queryResult = [d2, d1];
  const { result, rerender } = renderHook(() =>
    useInMemoryDocumentCache(queryResult, 4),
  );
  expect(result.current).toStrictEqual([d2, d1]);

  // When results disappear from the query they remain in the cache.
  queryResult = [d1];
  rerender();
  expect(result.current).toStrictEqual([d2, d1]);

  // Results stay in descending order even if query output is not in this order.
  queryResult = [d3, d2];
  rerender();
  expect(result.current).toStrictEqual([d3, d2, d1]);

  // The oldest drop out when going over the limit.
  queryResult = [d4, d5];
  rerender();
  expect(result.current).toStrictEqual([d5, d4, d3, d2]);

  // When passed undefined, return undefined.
  queryResult = undefined;
  rerender();
  expect(result.current).toStrictEqual(undefined);
});
