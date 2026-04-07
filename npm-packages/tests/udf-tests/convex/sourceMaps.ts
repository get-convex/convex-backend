/* eslint-disable @typescript-eslint/no-unused-vars */
import { query, mutation } from "./_generated/server";
import { doesntWork } from "./helpers.js";

interface _NotActuallyUsed {
  x: number;
}

function throwsTheError() {
  const _x = 1;
  throw new Error("Oh bother!");
}

function callsSomethingElse() {
  const _z = 3;
  throwsTheError();
  const _moreStuff = "line numbers";
  const _areGreat = "true that";
}

export const throwsError = query(async () => {
  const _soMuch = "unnecessaryCode";
  callsSomethingElse();
  const _a = "inThisFile";
});

export const throwsErrorInDep = query(async () => {
  const _thisIs = "alsoUnused";
  doesntWork();
});
