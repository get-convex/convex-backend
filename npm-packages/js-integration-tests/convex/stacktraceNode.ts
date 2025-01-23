"use node";
import http from "http";

import { action } from "./_generated/server";

export const simpleStackTrace = action(() => {
  return outer();
});

function outer() {
  return inner();
}

const inner = () => {
  return new Error().stack;
};

export const complexStackTrace = action(async () => {
  return await async1();
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

export const stackTraceUsedByProxyAgents = action(() => {
  return wontBeInTheStackTrace();
});

async function wontBeInTheStackTrace(): Promise<string> {
  return await new Promise((resolve) => {
    http.get("http://example.com", () => {
      resolve(new Error().stack || "");
    });
  });
}
