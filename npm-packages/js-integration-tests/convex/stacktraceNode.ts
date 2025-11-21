"use node";
import http from "http";

import { action } from "./_generated/server";

export const simpleStackTrace = action({
  args: {},
  handler: () => {
    return outer();
  },
});

function outer() {
  return inner();
}

const inner = () => {
  return new Error().stack;
};

export const complexStackTrace = action({
  args: {},
  handler: async () => {
    return await async1();
  },
});

const anonymousFunctions = [
  (f: () => string): string => {
    return anonymousFunctions[1](f);
  },
  (f: () => string): string => {
    return f();
  },
];

class Animal {
  move(): string {
    return new Error().stack!;
  }
}

async function async1() {
  const a = new Animal();
  return anonymousFunctions[0](() => a.move());
}

export const stackTraceUsedByProxyAgents = action({
  args: {},
  handler: () => {
    return wontBeInTheStackTrace();
  },
});

async function wontBeInTheStackTrace(): Promise<string> {
  return await new Promise((resolve) => {
    http.get("http://convex.dev", () => {
      resolve(new Error().stack || "");
    });
  });
}

export const errorWithMessage = action({
  args: {},
  handler: () => {
    return new Error("custom error message").stack;
  },
});
