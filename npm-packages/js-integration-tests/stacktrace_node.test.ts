import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

describe("Node.js user space stack traces", () => {
  let client: ConvexHttpClient;

  beforeEach(async () => {
    client = new ConvexHttpClient(deploymentUrl);
  });

  function canonicalize(s: string | null) {
    if (s === null) return null;
    let canonical = s;
    canonical = (canonical || "").replace(/:\d+:\d+/g, ":NUM:NUM");
    canonical = (canonical || "").replace(
      /_deps\/(node\/)?.*.js/g,
      "_deps/$1ABC.js",
    );
    return canonical;
  }

  test("simple stack trace", async () => {
    const result = await client.action(api.stacktraceNode.simpleStackTrace);
    expect(result).not.toBeNull();
    if (!result) return; // TypeScript guard

    // Verify the important parts of the stack trace - the user code frames
    expect(result).toContain("at inner (convex:/user/stacktraceNode.js:");
    expect(result).toContain("at outer (convex:/user/stacktraceNode.js:");

    // Verify the frames appear in the correct order (inner called by outer)
    const innerIndex = result.indexOf("at inner");
    const outerIndex = result.indexOf("at outer");
    expect(innerIndex).toBeLessThan(outerIndex);
  }, 20000);

  // Fails because our formatting doesn't match the default V8 implementation
  // yet. If we can find a way to use the default behavior then modify that
  // we'll be in business.
  // eslint-disable-next-line jest/no-disabled-tests
  test.skip("complex stack trace", async () => {
    const result = await client.action(api.stacktraceNode.complexStackTrace);
    const canonicalResult = canonicalize(result);
    const expected = `Error
    at Animal.move (convex:/user/stacktraceNode.js:16:12)
    at convex:/user/stacktraceNode.js:22:40
    at Array.anonymousFunctions (convex:/user/stacktraceNode.js:10:12)
    at Array.<anoniymous> (convex:/user/stacktraceNode.js:7:33)
    at async1 (convex:/user/stacktraceNode.js:22:31)
    at complexStackTrace (convex:/user/stacktraceNode.js:2:16)
    at convex:/user/stacktraceNode.js:25:19
    at invokeFunction (convex:/user/_deps/node/ABC.js:NUM:NUM)
    at invokeAction (convex:/user/_deps/node/ABC.js:NUM:NUM)
    at func.invokeAction (convex:/user/_deps/node/ABC.js:NUM:NUM)
    at executeInner (bundledFunctions.js:NUM:NUM)
    at async execute (bundledFunctions.js:NUM:NUM)
    at async invoke (bundledFunctions.js:NUM:NUM)
    at async main (bundledFunctions.js:NUM:NUM)`;
    const canonicalExpected = canonicalize(expected);

    expect(canonicalResult).toEqual(canonicalExpected);
  });

  test("stack trace used by the npm library proxy-agents", async () => {
    const result = await client.action(
      api.stacktraceNode.stackTraceUsedByProxyAgents,
    );
    const canonicalResult = canonicalize(result);
    const expected = `Error
    at convex:/user/stacktraceNode.js:19:15
    at onceWrapper (node:events:629:26)
    at emit (node:events:514:28)
    at parserOnIncomingClient (node:_http_client:700:27)
    at parserOnHeadersComplete (node:_http_common:119:17)
    at socketOnData (node:_http_client:541:22)
    at emit (node:events:514:28)
    at addChunk (node:internal/streams/readable:324:12)
    at readableAddChunk (node:internal/streams/readable:297:9)
    at Readable.push (node:internal/streams/readable:234:10)`;
    const canonicalExpected = canonicalize(expected);
    expect(canonicalResult).toEqual(canonicalExpected);
    // sometimes flakes at 20000ms
  }, 30000);
});
