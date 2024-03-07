import { httpRouter } from "convex/server";
import { imported } from "./http_no_default";
import { httpAction, query } from "./_generated/server";

// This file is used for testing the analyze logic for HTTP actions, including
// source mapping to the correct line numbers.
//
// HTTP actions used for testing the execution of HTTP actions
// (crates/isolate/src/tests/http_action.rs) should be defined in `http_action.ts`
// instead.

export const separateFunction = httpAction(async (_, _request: Request) => {
  throw new Error("Oh no!");
});

const http = httpRouter();
http.route({
  method: "GET",
  path: "/separate_function",
  handler: separateFunction,
});

http.route({
  method: "GET",
  path: "/inline",
  handler: httpAction(async (_, _request: Request) => {
    throw new Error("Oh no!");
  }),
});

http.route({
  method: "GET",
  path: "/imported",
  handler: imported,
});

export const myQuery = query((_) => {
  return "hello";
});

export default http;
