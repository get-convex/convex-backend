import { fibonacci } from "./a.js";
import fs from "fs";

async function probablyNot() {
  throw new Error("such is life");
}

async function doesThisWork() {
  await probablyNot();
}

export const throwError = {
  isAction: true,

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  invokeAction: async (requestId, args) => {
    const files = await fs.promises.readdir(".");
    console.log("files", files);
    await doesThisWork();
  },
};

export const getNumber = {
  isAction: true,
  invokeAction: async (requestId, args) => {
    // This is invalid.
    return fibonacci(Number(args));
  },
};

export default {
  isAction: true,
  invokeAction: async (requestId, args) => {
    console.log("Computing...");
    return JSON.stringify(fibonacci(Number(args)));
  },
};
