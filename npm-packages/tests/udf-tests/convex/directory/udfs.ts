import { fibonacci } from "../helpers.js";
import { query } from "../_generated/server";

export const f = query(function f(_, { a, b }: { a: number; b: number }) {
  console.log("Running f!");
  return fibonacci(a) + fibonacci(b);
});

export const g = query(function g(_, { a, b }: { a: number; b: number }) {
  console.warn("Running g :(");
  return fibonacci(a) - fibonacci(b);
});

export const returnsUndefined = query(() => {
  console.info("This function doesn't return anything");
});

export const pseudoRandom = query(() => {
  return Math.random();
});

export const noop = query(() => {
  // intentional noop.
});

export const usesDate = query(() => {
  const dateCall = Date();
  const newDateYear = new Date().getUTCFullYear();
  const dateNow = Date.now();
  return { dateCall, newDateYear, dateNow };
});
