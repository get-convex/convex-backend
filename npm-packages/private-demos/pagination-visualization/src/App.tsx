import { NAMES } from "./names";
import { XMarkIcon } from "@heroicons/react/24/outline";
import {
  useMutation,
  usePaginatedQueryInternal,
  page,
  includePage,
} from "convex/react";
import { api } from "../convex/_generated/api";
import { Fragment } from "react";
import { Infer } from "convex/values";
import { paginationOptsValidator } from "convex/server";

export type FakeDocument = { _id: string; name: string };

export default function App() {
  const {
    user: { results, status, loadMore },
    internal,
  } = usePaginatedQueryInternal(
    api.names.getPeople,
    {},
    { initialNumItems: 5, [includePage]: true },
  );

  const add = useMutation(api.names.addPerson);
  const remove = useMutation(api.names.deletePerson);
  const seed = useMutation(api.names.seed);

  function addNameAtIndex(currentIndex: number) {
    const names = results;

    const thisNameListIndex =
      currentIndex === -1
        ? -1
        : currentIndex === names.length
          ? names.length
          : binarySearch(NAMES, names[currentIndex].name);

    const nextNameListIndex =
      currentIndex === names.length - 1
        ? NAMES.length
        : binarySearch(NAMES, names[currentIndex + 1].name);

    if (thisNameListIndex + 1 === nextNameListIndex) {
      return;
    }

    const name = NAMES[Math.floor((thisNameListIndex + nextNameListIndex) / 2)];
    void add({ name });
  }

  const pages = Object.entries(internal.state.queries).map(([key, value]) => {
    const resultsWithPage = results as unknown as {
      [page]: string;
    }[];

    return {
      firstIndex: resultsWithPage.findIndex((r) => r[page] === key),
      lastIndex: resultsWithPage.findLastIndex((r) => r[page] === key),
      key,
      value,
    };
  });
  const emptyPages = pages.filter((p) => p.firstIndex === -1);

  return (
    <div className="p-8">
      <header className="flex gap-4 flex-wrap justify-between items-center">
        <h1 className="text-4xl font-semibold tracking-tight">
          Convex Pagination Visualization
        </h1>
        <button
          onClick={() => {
            const expectedNames = 20;
            const randomIndexes = Array.from(
              { length: NAMES.length },
              (_, i) => i,
            )
              .toSorted(() => Math.random() - 0.5)
              .splice(0, expectedNames)
              .toSorted();

            void seed({ names: randomIndexes.map((i) => NAMES[i]) });
          }}
          className="bg-gradient-to-b from-blue-500 to-blue-600 text-white px-4 py-2 rounded-lg shadow-md hover:from-blue-600 hover:to-blue-700 transition-all duration-300 ease-in-out transform disabled:from-gray-100 disabled:to-gray-100 disabled:text-gray-400"
        >
          Seed
        </button>
      </header>

      <div className="gap-8">
        <div className="flex flex-col gap-4 my-8 flex-2">
          <h3 className="text-2xl font-black">Paginated</h3>
          <div className="bg-slate-200 p-4 rounded-xl grid grid-cols-2 gap-x-4">
            <div className="flex flex-col">
              <AddButton
                onClick={() => {
                  addNameAtIndex(-1);
                }}
              />
              {results.map((value, index) => (
                <Row
                  key={value._id}
                  value={value}
                  onRemove={() => void remove({ id: value._id })}
                  onAdd={() => addNameAtIndex(index)}
                />
              ))}
            </div>

            <div>
              <div className="h-3"></div>
              <div className="relative">
                {pages
                  .filter((p) => p.firstIndex !== -1)
                  .map(({ key, value, firstIndex, lastIndex }) => {
                    const HEIGHT_PER_ROW = 20 + 25;

                    return (
                      <div
                        key={key}
                        className="absolute left-0"
                        style={{
                          top: firstIndex * HEIGHT_PER_ROW,
                          height: (lastIndex - firstIndex + 1) * HEIGHT_PER_ROW,
                        }}
                      >
                        <PageDetails
                          pageKey={key}
                          paginationOpts={value.args.paginationOpts}
                        />
                      </div>
                    );
                  })}
              </div>
            </div>

            <div>
              <button
                onClick={() => loadMore(5)}
                className="w-full bg-gradient-to-b from-blue-500 to-blue-600 text-white px-4 py-2 rounded-lg shadow-md hover:from-blue-600 hover:to-blue-700 transition-all duration-300 ease-in-out transform disabled:from-gray-100 disabled:to-gray-100 disabled:text-gray-400"
                disabled={status !== "CanLoadMore"}
              >
                Load 5 more
              </button>
            </div>

            {emptyPages.length > 0 && (
              <div className="border-y-4 border-red-300 py-4 mt-6 rounded flex flex-col gap-2">
                <p className="">
                  The following pages don’t have any rows associated with them:
                </p>

                {emptyPages.map(({ key, value }) => (
                  <PageDetails
                    key={key}
                    pageKey={key}
                    paginationOpts={value.args.paginationOpts}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="h-36"></div>
      <div className="border-t backdrop-blur bg-white/20 fixed bottom-0 inset-x-0 p-4 flex items-center">
        <div className="flex-1">Status = {status}</div>
      </div>
    </div>
  );
}

function Row({
  value,
  onRemove,
  onAdd,
}: {
  value: FakeDocument;
  onRemove: () => void;
  onAdd: () => void;
}) {
  return (
    <>
      <div className="bg-white shadow rounded text-xs uppercase tracking-widest px-2 animate-[appear_0.5s_ease-in-out] font-semibold relative flex items-center gap-1.5 h-5">
        <div
          className={`size-3 rounded-full border-black/20 border ${generateRandomColorFromName(value.name)}`}
        />
        <div className="flex-1">{value.name}</div>

        {"__pageKey" in value && typeof value.__pageKey === "string" && (
          <div
            className={`size-2 rotate-45 border-black/20 border ${generateRandomColorFromName(value.__pageKey)}`}
          />
        )}

        <button
          className="size-5 flex items-center justify-center cursor-pointer text-slate-500 hover:text-red-600"
          onClick={() => {
            onRemove();
          }}
        >
          <XMarkIcon className="size-4" />
        </button>
      </div>
      <AddButton
        onClick={() => {
          onAdd();
        }}
      />
    </>
  );
}

function AddButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      className="w-full cursor-pointer flex items-center text-slate-400 hover:text-slate-800 gap-2 py-3"
      onClick={onClick}
    >
      <div className="flex-grow border-t border-current opacity-50"></div>
    </button>
  );
}

function PageDetails({
  pageKey,
  paginationOpts,
}: {
  pageKey: string;
  paginationOpts: Infer<typeof paginationOptsValidator>;
}) {
  return (
    <div className="flex gap-2 size-full">
      <div
        className={`w-1.5 my-1 shrink-0 rounded-full ${generateRandomColorFromName(pageKey)}`}
      ></div>

      <div className="overflow-auto">
        <div className="p-2">
          <header className="text-xl flex items-center gap-2 font-semibold mb-1">
            <div
              className={`size-3 rotate-45 border-black/20 border ${generateRandomColorFromName(pageKey)}`}
            />
            Page {pageKey}
          </header>
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 text-sm">
            {Object.entries(paginationOpts).map(([key, value]) => (
              <Fragment key={key}>
                <dt className="font-medium">{key}</dt>
                <dd className="font-mono truncate flex gap-2 items-center">
                  <span>
                    {(key === "cursor" || key === "endCursor") && "🔒 "}
                  </span>
                  {(key === "cursor" || key === "endCursor") &&
                    value !== null && (
                      <div
                        className={`size-3 shrink-0 border-black/20 border ${generateRandomColorFromName(value.toString())}`}
                      />
                    )}
                  {value === null ? "null" : value}
                </dd>
              </Fragment>
            ))}
          </dl>
        </div>
      </div>
    </div>
  );
}

function binarySearch(arr: string[], target: string): number {
  let startIndex = 0;
  let endIndex = arr.length - 1;

  while (startIndex <= endIndex) {
    const midIndex = Math.floor((startIndex + endIndex) / 2);

    if (arr[midIndex] === target) {
      return midIndex; // Target found, return its index
    } else if (arr[midIndex] < target) {
      startIndex = midIndex + 1;
    } else {
      endIndex = midIndex - 1;
    }
  }

  return -1; // Target not found in the array
}

function generateRandomColorFromName(name: string): string {
  const COLORS = [
    "bg-amber-300",
    "bg-blue-300",
    "bg-cyan-300",
    "bg-emerald-300",
    "bg-fuchsia-300",
    "bg-green-300",
    "bg-indigo-300",
    "bg-lime-300",
    "bg-orange-300",
    "bg-pink-300",
    "bg-purple-300",
    "bg-red-300",
    "bg-rose-300",
    "bg-sky-300",
    "bg-teal-300",
    "bg-violet-300",
    "bg-yellow-300",

    "bg-amber-400",
    "bg-blue-400",
    "bg-cyan-400",
    "bg-emerald-400",
    "bg-fuchsia-400",
    "bg-green-400",
    "bg-indigo-400",
    // "bg-lime-400",
    "bg-orange-400",
    "bg-pink-400",
    "bg-purple-400",
    "bg-red-400",
    "bg-rose-400",
    "bg-sky-400",
    "bg-teal-400",
    "bg-violet-400",
    "bg-yellow-400",

    "bg-amber-500",
    "bg-blue-500",
    "bg-cyan-500",
    "bg-emerald-500",
    "bg-fuchsia-500",
    "bg-green-500",
    "bg-indigo-500",
    "bg-lime-500",
    "bg-orange-500",
    "bg-pink-500",
    "bg-purple-500",
    "bg-red-500",
    "bg-rose-500",
    "bg-sky-500",
    "bg-teal-500",
    "bg-violet-500",
    "bg-yellow-500",

    "bg-amber-600",
    "bg-blue-600",
    "bg-cyan-600",
    "bg-emerald-600",
    "bg-fuchsia-600",
    "bg-green-600",
    "bg-indigo-600",
    "bg-lime-600",
    "bg-orange-600",
    "bg-pink-600",
    "bg-purple-600",
    "bg-red-600",
    "bg-rose-600",
    "bg-sky-600",
    "bg-teal-600",
    "bg-violet-600",
    "bg-yellow-600",
  ];
  const hash = name
    .split("")
    .reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return COLORS[hash % COLORS.length];
}
