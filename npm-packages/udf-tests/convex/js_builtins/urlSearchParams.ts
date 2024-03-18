// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { query } from "../_generated/server";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

function urlSearchParamsWithMultipleSpaces() {
  const init = { str: "this string has spaces in it" };
  const searchParams = new URLSearchParams(init).toString();
  assert.strictEqual(searchParams, "str=this+string+has+spaces+in+it");
}

function urlSearchParamsWithExclamation() {
  const init = [["str", "hello, world!"]];
  const searchParams = new URLSearchParams(init).toString();
  assert.strictEqual(searchParams, "str=hello%2C+world%21");
}

function urlSearchParamsWithQuotes() {
  const init = [["str", "'hello world'"]];
  const searchParams = new URLSearchParams(init).toString();
  assert.strictEqual(searchParams, "str=%27hello+world%27");
}

function urlSearchParamsWithBraket() {
  const init = [["str", "(hello world)"]];
  const searchParams = new URLSearchParams(init).toString();
  assert.strictEqual(searchParams, "str=%28hello+world%29");
}

function urlSearchParamsWithTilde() {
  const init = [["str", "hello~world"]];
  const searchParams = new URLSearchParams(init).toString();
  assert.strictEqual(searchParams, "str=hello%7Eworld");
}

function urlSearchParamsInitString() {
  const init = "c=4&a=2&b=3&%C3%A1=1";
  const searchParams = new URLSearchParams(init);
  assert(
    init === searchParams.toString(),
    "The init query string does not match",
  );
}

function urlSearchParamsInitStringWithPlusCharacter() {
  let params = new URLSearchParams("q=a+b");
  assert.strictEqual(params.toString(), "q=a+b");
  assert.strictEqual(params.get("q"), "a b");

  params = new URLSearchParams("q=a+b+c");
  assert.strictEqual(params.toString(), "q=a+b+c");
  assert.strictEqual(params.get("q"), "a b c");
}

function urlSearchParamsInitStringWithMalformedParams() {
  // These test cases are copied from Web Platform Tests
  // https://github.com/web-platform-tests/wpt/blob/54c6d64/url/urlsearchparams-constructor.any.js#L60-L80
  let params = new URLSearchParams("id=0&value=%");
  assert(params !== null, "constructor returned non-null value.");
  assert(params.has("id"), 'Search params object has name "id"');
  assert(params.has("value"), 'Search params object has name "value"');
  assert.strictEqual(params.get("id"), "0");
  assert.strictEqual(params.get("value"), "%");

  params = new URLSearchParams("b=%2sf%2a");
  assert(params !== null, "constructor returned non-null value.");
  assert(params.has("b"), 'Search params object has name "b"');
  assert.strictEqual(params.get("b"), "%2sf*");

  params = new URLSearchParams("b=%2%2af%2a");
  assert(params !== null, "constructor returned non-null value.");
  assert(params.has("b"), 'Search params object has name "b"');
  assert.strictEqual(params.get("b"), "%2*f*");

  params = new URLSearchParams("b=%%2a");
  assert(params !== null, "constructor returned non-null value.");
  assert(params.has("b"), 'Search params object has name "b"');
  assert.strictEqual(params.get("b"), "%*");
}

function urlSearchParamsInitIterable() {
  const init = [
    ["a", "54"],
    ["b", "true"],
  ];
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.toString(), "a=54&b=true");
}

function urlSearchParamsInitRecord() {
  const init = { a: "54", b: "true" };
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.toString(), "a=54&b=true");
}

function urlSearchParamsInit() {
  const params1 = new URLSearchParams("a=b");
  assert.strictEqual(params1.toString(), "a=b");
  const params2 = new URLSearchParams(params1);
  assert.strictEqual(params2.toString(), "a=b");
}

function urlSearchParamsAppendSuccess() {
  const searchParams = new URLSearchParams();
  searchParams.append("a", "true");
  assert.strictEqual(searchParams.toString(), "a=true");
}

function urlSearchParamsDeleteSuccess() {
  const init = "a=54&b=true";
  const searchParams = new URLSearchParams(init);
  searchParams.delete("b");
  assert.strictEqual(searchParams.toString(), "a=54");
}

function urlSearchParamsGetAllSuccess() {
  const init = "a=54&b=true&a=true";
  const searchParams = new URLSearchParams(init);
  assert.deepEqual(searchParams.getAll("a"), ["54", "true"]);
  assert.deepEqual(searchParams.getAll("b"), ["true"]);
  assert.deepEqual(searchParams.getAll("c"), []);
}

function urlSearchParamsGetSuccess() {
  const init = "a=54&b=true&a=true";
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.get("a"), "54");
  assert.strictEqual(searchParams.get("b"), "true");
  assert.strictEqual(searchParams.get("c"), null);
}

function urlSearchParamsHasSuccess() {
  const init = "a=54&b=true&a=true";
  const searchParams = new URLSearchParams(init);
  assert(searchParams.has("a"));
  assert(searchParams.has("b"));
  assert(!searchParams.has("c"));
}

// function urlSearchParamsSetReplaceFirstAndRemoveOthers() {
//   const init = "a=54&b=true&a=true";
//   const searchParams = new URLSearchParams(init);
//   searchParams.set("a", "false");
//   assert.strictEqual(searchParams.toString(), "a=false&b=true");
// }

function urlSearchParamsSetAppendNew() {
  const init = "a=54&b=true&a=true";
  const searchParams = new URLSearchParams(init);
  searchParams.set("c", "foo");
  assert.strictEqual(searchParams.toString(), "a=54&b=true&a=true&c=foo");
}

function urlSearchParamsSortSuccess() {
  const init = "c=4&a=2&b=3&a=1";
  const searchParams = new URLSearchParams(init);
  searchParams.sort();
  assert.strictEqual(searchParams.toString(), "a=2&a=1&b=3&c=4");
}

function urlSearchParamsForEachSuccess() {
  const init = [
    ["a", "54"],
    ["b", "true"],
  ];
  const searchParams = new URLSearchParams(init);
  let callNum = 0;
  searchParams.forEach((value, key, parent) => {
    assert(searchParams === parent);
    assert.strictEqual(value, init[callNum][1]);
    assert.strictEqual(key, init[callNum][0]);
    callNum++;
  });
  assert.strictEqual(callNum, init.length);
}

function urlSearchParamsMissingName() {
  const init = "=4";
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.get(""), "4");
  assert.strictEqual(searchParams.toString(), "=4");
}

function urlSearchParamsMissingValue() {
  const init = "4=";
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.get("4"), "");
  assert.strictEqual(searchParams.toString(), "4=");
}

function urlSearchParamsMissingEqualSign() {
  const init = "4";
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.get("4"), "");
  assert.strictEqual(searchParams.toString(), "4=");
}

function urlSearchParamsMissingPair() {
  const init = "c=4&&a=54&";
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.toString(), "c=4&a=54");
}

function urlSearchParamsForShortEncodedChar() {
  const init = { linefeed: "\n", tab: "\t" };
  const searchParams = new URLSearchParams(init);
  assert.strictEqual(searchParams.toString(), "linefeed=%0A&tab=%09");
}

// If pair does not contain exactly two items, then throw a TypeError.
// ref https://url.spec.whatwg.org/#interface-urlsearchparams
function urlSearchParamsShouldThrowTypeError() {
  assert.throws(() => new URLSearchParams([["1"]]), TypeError);
  assert.throws(() => new URLSearchParams([["1", "2", "3"]]), TypeError);
}

// function urlSearchParamsAppendArgumentsCheck() {
//   const methodRequireOneParam = ["delete", "getAll", "get", "has", "forEach"];

//   const methodRequireTwoParams = ["append", "set"];

//   methodRequireOneParam
//     .concat(methodRequireTwoParams)
//     .forEach((method: string) => {
//       const searchParams = new URLSearchParams();
//       let hasThrown = 0;
//       try {
//         // deno-lint-ignore no-explicit-any
//         (searchParams as any)[method]();
//         hasThrown = 1;
//       } catch (err) {
//         if (err instanceof TypeError) {
//           hasThrown = 2;
//         } else {
//           hasThrown = 3;
//         }
//       }
//       assert.strictEqual(hasThrown, 2);
//     });

//   methodRequireTwoParams.forEach((method: string) => {
//     const searchParams = new URLSearchParams();
//     let hasThrown = 0;
//     try {
//       // deno-lint-ignore no-explicit-any
//       (searchParams as any)[method]("foo");
//       hasThrown = 1;
//     } catch (err) {
//       if (err instanceof TypeError) {
//         hasThrown = 2;
//       } else {
//         hasThrown = 3;
//       }
//     }
//     assert.strictEqual(hasThrown, 2);
//   });
// }

// ref: https://github.com/web-platform-tests/wpt/blob/master/url/urlsearchparams-delete.any.js
function urlSearchParamsDeletingAppendedMultiple() {
  const params = new URLSearchParams();
  params.append("first", "1");
  assert(params.has("first"));
  assert.strictEqual(params.get("first"), "1");
  params.delete("first");
  assert.strictEqual(params.has("first"), false);
  params.append("first", "1");
  params.append("first", "10");
  params.delete("first");
  assert.strictEqual(params.has("first"), false);
}

// // ref: https://github.com/web-platform-tests/wpt/blob/master/url/urlsearchparams-constructor.any.js#L176-L182
// function urlSearchParamsCustomSymbolIterator() {
//   const params = new URLSearchParams();
//   params[Symbol.iterator] = function* (): IterableIterator<[string, string]> {
//     yield ["a", "b"];
//   };
//   const params1 = new URLSearchParams(params as unknown as string[][]);
//   assert.strictEqual(params1.get("a"), "b");
// }

// function urlSearchParamsCustomSymbolIteratorWithNonStringParams() {
//   const params = {};
//   // deno-lint-ignore no-explicit-any
//   (params as any)[Symbol.iterator] = function* (): IterableIterator<
//     [number, number]
//   > {
//     yield [1, 2];
//   };
//   const params1 = new URLSearchParams(params as unknown as string[][]);
//   assert.strictEqual(params1.get("1"), "2");
// }

// // If a class extends URLSearchParams, override one method should not change another's behavior.
// function urlSearchParamsOverridingAppendNotChangeConstructorAndSet() {
//   let overridedAppendCalled = 0;
//   class CustomSearchParams extends URLSearchParams {
//     append(name: string, value: string) {
//       ++overridedAppendCalled;
//       super.append(name, value);
//     }
//   }
//   new CustomSearchParams("foo=bar");
//   new CustomSearchParams([["foo", "bar"]]);
//   new CustomSearchParams(new CustomSearchParams({ foo: "bar" }));
//   new CustomSearchParams().set("foo", "bar");
//   assert.strictEqual(overridedAppendCalled, 0);
// }

function urlSearchParamsOverridingEntriesNotChangeForEach() {
  class CustomSearchParams extends URLSearchParams {
    *entries(): IterableIterator<[string, string]> {
      yield* [];
    }
  }
  let loopCount = 0;
  const params = new CustomSearchParams({ foo: "bar" });
  params.forEach(() => void ++loopCount);
  assert.strictEqual(loopCount, 1);
}

function urlSearchParamsNonString() {
  // @ts-expect-error intentional error
  const params = new URLSearchParams({ a: 123 });
  assert.strictEqual(params.toString(), "a=123");
}

export default query(async () => {
  return await wrapInTests({
    urlSearchParamsWithMultipleSpaces,
    urlSearchParamsWithExclamation,
    urlSearchParamsWithQuotes,
    urlSearchParamsWithBraket,
    urlSearchParamsWithTilde,
    urlSearchParamsInitString,
    urlSearchParamsInitStringWithPlusCharacter,
    urlSearchParamsInitStringWithMalformedParams,
    urlSearchParamsInitIterable,
    urlSearchParamsInitRecord,
    urlSearchParamsInit,
    urlSearchParamsAppendSuccess,
    urlSearchParamsDeleteSuccess,
    urlSearchParamsGetAllSuccess,
    urlSearchParamsGetSuccess,
    urlSearchParamsHasSuccess,
    // urlSearchParamsSetReplaceFirstAndRemoveOthers,
    urlSearchParamsSetAppendNew,
    urlSearchParamsSortSuccess,
    urlSearchParamsForEachSuccess,
    urlSearchParamsMissingName,
    urlSearchParamsMissingValue,
    urlSearchParamsMissingEqualSign,
    urlSearchParamsMissingPair,
    urlSearchParamsForShortEncodedChar,
    urlSearchParamsShouldThrowTypeError,
    // urlSearchParamsAppendArgumentsCheck
    urlSearchParamsDeletingAppendedMultiple,
    // urlSearchParamsCustomSymbolIterator
    // urlSearchParamsCustomSymbolIteratorWithNonStringParams
    // urlSearchParamsOverridingAppendNotChangeConstructorAndSet
    urlSearchParamsOverridingEntriesNotChangeForEach,
    urlSearchParamsNonString,
  });
});
